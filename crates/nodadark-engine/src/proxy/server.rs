// nodadark-engine/src/proxy/server.rs
// Serveur proxy HTTP/HTTPS avec interception MITM

use crate::{
    proxy::{cert::CertificateAuthority, ProxyState},
    rules::RulesEngine,
    EngineEvent, InterceptedRequest, ProxyConfig, RequestState,
};
use anyhow::Result;
use bytes::Bytes;
use futures::TryFutureExt;
use hyper::{
    body::to_bytes,
    client::HttpConnector,
    server::conn::Http,
    service::service_fn,
    Body, Client, Method, Request, Response, StatusCode, Uri,
};
use hyper_rustls::HttpsConnector;
use rustls::{ServerConfig, ServerName};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

type HttpClient = Client<HttpsConnector<HttpConnector>>;

pub async fn run_proxy(
    config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    ca: Arc<CertificateAuthority>,
    tx: broadcast::Sender<EngineEvent>,
) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", config.bind, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Proxy en écoute sur {addr}");

    // Client HTTP(S) pour les requêtes sortantes
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build();
    let client: HttpClient = Client::builder().build(https);

    // Moteur de règles
    let rules = Arc::new(RulesEngine::load_or_default(&config));

    loop {
        let (stream, client_addr) = listener.accept().await?;
        tracing::debug!("Nouvelle connexion depuis {client_addr}");

        let config = config.clone();
        let state  = state.clone();
        let ca     = ca.clone();
        let tx     = tx.clone();
        let client = client.clone();
        let rules  = rules.clone();

        tokio::spawn(async move {
            if let Err(e) =
                handle_connection(stream, config, state, ca, tx, client, rules).await
            {
                tracing::debug!("Erreur connexion client: {e}");
            }
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    ca: Arc<CertificateAuthority>,
    tx: broadcast::Sender<EngineEvent>,
    client: HttpClient,
    rules: Arc<RulesEngine>,
) -> Result<()> {
    // Lire la première requête pour déterminer si c'est un CONNECT (HTTPS)
    // ou une requête HTTP normale
    let service = service_fn(move |req: Request<Body>| {
        let config = config.clone();
        let state  = state.clone();
        let ca     = ca.clone();
        let tx     = tx.clone();
        let client = client.clone();
        let rules  = rules.clone();

        async move {
            if req.method() == Method::CONNECT {
                handle_https_tunnel(req, config, state, ca, tx, client, rules).await
            } else {
                handle_http_request(req, config, state, tx, client, rules).await
            }
        }
    });

    Http::new()
        .serve_connection(stream, service)
        .with_upgrades()
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

// ────────────────────────────────────────────────────────────
//  Tunnel HTTPS (méthode CONNECT)
// ────────────────────────────────────────────────────────────

async fn handle_https_tunnel(
    req: Request<Body>,
    _config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    ca: Arc<CertificateAuthority>,
    tx: broadcast::Sender<EngineEvent>,
    client: HttpClient,
    rules: Arc<RulesEngine>,
) -> Result<Response<Body>, hyper::Error> {
    let host = req
        .uri()
        .authority()
        .map(|a| a.to_string())
        .unwrap_or_default();

    tracing::debug!("CONNECT vers {host}");

    // Répondre 200 Connection Established pour accepter le tunnel
    tokio::task::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                // Obtenir le certificat pour cet hôte
                let tls_cert = match ca.get_or_create_for_host(&host) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Erreur cert pour {host}: {e}");
                        return;
                    }
                };

                // Configurer TLS serveur (vers le client)
                let server_config = build_server_tls_config(&tls_cert);
                let acceptor = TlsAcceptor::from(Arc::new(server_config));

                match acceptor.accept(upgraded).await {
                    Ok(tls_stream) => {
                        // Servir les requêtes HTTPS via ce flux TLS
                        let host_clone = host.clone();
                        let service = service_fn(move |mut inner_req: Request<Body>| {
                            let host = host_clone.clone();
                            let state = state.clone();
                            let tx    = tx.clone();
                            let client = client.clone();
                            let rules  = rules.clone();

                            // Reconstruire l'URL complète
                            if inner_req.uri().scheme().is_none() {
                                let full_uri = format!(
                                    "https://{}{}",
                                    host,
                                    inner_req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/")
                                );
                                if let Ok(uri) = full_uri.parse::<Uri>() {
                                    *inner_req.uri_mut() = uri;
                                }
                            }

                            async move {
                                handle_http_request(inner_req, 
                                    Arc::new(ProxyConfig::default()),
                                    state, tx, client, rules).await
                            }
                        });

                        if let Err(e) = Http::new()
                            .serve_connection(tls_stream, service)
                            .with_upgrades()
                            .await
                        {
                            tracing::debug!("TLS connection error pour {host}: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("TLS handshake échoué pour {host}: {e}");
                    }
                }
            }
            Err(e) => tracing::error!("Upgrade échoué: {e}"),
        }
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap())
}

// ────────────────────────────────────────────────────────────
//  Requête HTTP ordinaire
// ────────────────────────────────────────────────────────────

async fn handle_http_request(
    req: Request<Body>,
    _config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    tx: broadcast::Sender<EngineEvent>,
    client: HttpClient,
    rules: Arc<RulesEngine>,
) -> Result<Response<Body>, hyper::Error> {
    let id        = Uuid::new_v4().to_string();
    let method    = req.method().to_string();
    let uri       = req.uri().clone();
    let url       = uri.to_string();
    let host      = uri.host().unwrap_or("unknown").to_string();
    let path      = uri.path_and_query().map(|p| p.to_string()).unwrap_or_default();
    let tls       = uri.scheme_str() == Some("https");
    let http_ver  = format!("{:?}", req.version());
    let timestamp = chrono::Utc::now();

    let start = Instant::now();

    // Capturer les headers
    let req_headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Lire le body de la requête
    let (req_parts, req_body) = req.into_parts();
    let req_body_bytes = to_bytes(req_body).await.unwrap_or_default();
    let req_body_data = if req_body_bytes.is_empty() {
        None
    } else {
        Some(req_body_bytes.to_vec())
    };

    // Enregistrer la requête entrante
    let intercepted = InterceptedRequest {
        id: id.clone(),
        method: method.clone(),
        url: url.clone(),
        host: host.clone(),
        path: path.clone(),
        http_version: http_ver,
        request_headers: req_headers.clone(),
        request_body: req_body_data.clone(),
        response_status: None,
        response_headers: vec![],
        response_body: None,
        duration_ms: None,
        timestamp,
        state: RequestState::Pending,
        tls,
        error: None,
    };
    state.upsert(intercepted);

    // Émettre l'événement "nouvelle requête"
    let _ = tx.send(EngineEvent::Request {
        id: id.clone(),
        method: method.clone(),
        url: url.clone(),
        host: host.clone(),
        timestamp,
        tls,
    });

    // Appliquer les règles (drop, modifier, etc.)
    let action = rules.evaluate(&host, &path, &req_headers);
    if let crate::rules::RuleAction::Drop = action {
        let _ = tx.send(EngineEvent::Dropped { id: id.clone() });
        state.drop_request(&id);
        return Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("NodaDark: Requête droppée par règle."))
            .unwrap());
    }

    // Reconstruire la requête pour la transmettre
    let mut forward_req = Request::from_parts(req_parts, Body::from(req_body_bytes));
    // Appliquer les modifications de règles sur les headers
    if let crate::rules::RuleAction::ModifyHeaders(mods) = action {
        for (k, v) in &mods {
            if let (Ok(name), Ok(val)) = (
                k.parse::<hyper::header::HeaderName>(),
                v.parse::<hyper::header::HeaderValue>(),
            ) {
                forward_req.headers_mut().insert(name, val);
            }
        }
    }

    // Envoyer la requête au serveur cible
    let response = match client.request(forward_req).await {
        Ok(r) => r,
        Err(e) => {
            let err_str = e.to_string();
            tracing::debug!("Erreur upstream pour {url}: {err_str}");
            let _ = tx.send(EngineEvent::RequestError {
                id: id.clone(),
                error: err_str.clone(),
            });
            if let Some(mut req_entry) = state.requests.get_mut(&id) {
                req_entry.state = RequestState::Error;
                req_entry.error = Some(err_str.clone());
            }
            // Fail-open : retourner une erreur lisible
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(format!("NodaDark: {err_str}")))
                .unwrap());
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();
    let resp_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Lire le body de la réponse (attention aux gros fichiers)
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let (resp_parts, resp_body) = response.into_parts();
    let resp_body_bytes = to_bytes(resp_body).await.unwrap_or_default();
    let resp_body_data = if resp_body_bytes.is_empty() {
        None
    } else {
        Some(resp_body_bytes.to_vec())
    };
    let size = resp_body_data.as_ref().map(|b| b.len()).unwrap_or(0);

    // Mettre à jour la requête avec la réponse
    if let Some(mut req_entry) = state.requests.get_mut(&id) {
        req_entry.response_status  = Some(status);
        req_entry.response_headers = resp_headers.clone();
        req_entry.response_body    = resp_body_data.clone();
        req_entry.duration_ms      = Some(duration_ms);
        req_entry.state            = RequestState::Complete;
    }

    let _ = tx.send(EngineEvent::Response {
        id,
        status,
        duration_ms,
        size,
    });

    // Renvoyer la réponse au client
    Ok(Response::from_parts(
        resp_parts,
        Body::from(resp_body_bytes),
    ))
}

fn build_server_tls_config(cert: &crate::proxy::cert::CachedCert) -> ServerConfig {
    let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.private_key.clone())
        .expect("Erreur configuration TLS serveur");
    config.alpn_protocols = vec![b"http/1.1".to_vec(), b"h2".to_vec()];
    config
}
