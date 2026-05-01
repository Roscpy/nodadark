// nodadark-engine/src/proxy/cert.rs
// Génération du CA racine et des certificats par-hôte (MITM)

use anyhow::{Context, Result};
use dashmap::DashMap;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose,
    IsCa, KeyUsagePurpose, SanType,
};
use rustls::{Certificate as RustlsCert, PrivateKey};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

pub struct CertificateAuthority {
    ca_cert: Certificate,
    cache: DashMap<String, Arc<CachedCert>>,
    cert_dir: PathBuf,
}

pub struct CachedCert {
    pub cert_chain: Vec<RustlsCert>,
    pub private_key: PrivateKey,
}

impl CertificateAuthority {
    pub async fn load_or_create(cert_dir: &str) -> Result<Self> {
        let cert_dir = PathBuf::from(cert_dir);
        fs::create_dir_all(&cert_dir).await?;

        let ca_cert_path = cert_dir.join("nodadark-ca.crt");
        let ca_key_path  = cert_dir.join("nodadark-ca.key");

        let ca_cert = if ca_cert_path.exists() && ca_key_path.exists() {
            tracing::debug!("Chargement du CA existant depuis {}", cert_dir.display());
            Self::load_ca(&ca_key_path).await?
        } else {
            tracing::info!("Génération d'un nouveau CA NodaDark...");
            let cert = Self::generate_ca()?;
            fs::write(&ca_cert_path, cert.serialize_pem()?)
                .await
                .context("Impossible d'écrire le certificat CA")?;
            fs::write(&ca_key_path, cert.serialize_private_key_pem())
                .await
                .context("Impossible d'écrire la clé privée CA")?;
            tracing::info!("CA généré : {}", ca_cert_path.display());
            cert
        };

        Ok(Self { ca_cert, cache: DashMap::new(), cert_dir })
    }

    fn generate_ca() -> Result<Certificate> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params.distinguished_name.push(DnType::CommonName, "NodaDark CA");
        params.distinguished_name.push(DnType::OrganizationName, "NodaDark Security Tools");
        params.distinguished_name.push(DnType::CountryName, "FR");
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after  = rcgen::date_time_ymd(2034, 1, 1);
        Certificate::from_params(params).context("Génération CA échouée")
    }

    // rcgen 0.12 ne supporte pas from_ca_cert_pem
    // On recrée les params avec la même clé privée sauvegardée
    async fn load_ca(key_path: &PathBuf) -> Result<Certificate> {
        let key_pem  = fs::read_to_string(key_path).await?;
        let key_pair = rcgen::KeyPair::from_pem(&key_pem)
            .context("Lecture clé privée CA échouée")?;

        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params.distinguished_name.push(DnType::CommonName, "NodaDark CA");
        params.distinguished_name.push(DnType::OrganizationName, "NodaDark Security Tools");
        params.distinguished_name.push(DnType::CountryName, "FR");
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after  = rcgen::date_time_ymd(2034, 1, 1);
        params.key_pair   = Some(key_pair);

        Certificate::from_params(params).context("Chargement CA échoué")
    }

    pub fn get_or_create_for_host(&self, host: &str) -> Result<Arc<CachedCert>> {
        let hostname = host.split(':').next().unwrap_or(host).to_string();
        if let Some(cached) = self.cache.get(&hostname) {
            return Ok(cached.clone());
        }
        let cert = self.create_host_cert(&hostname)?;
        let arc  = Arc::new(cert);
        self.cache.insert(hostname, arc.clone());
        Ok(arc)
    }

    fn create_host_cert(&self, hostname: &str) -> Result<CachedCert> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::NoCa;
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.distinguished_name.push(DnType::CommonName, hostname);
        params.subject_alt_names = vec![SanType::DnsName(hostname.to_string())];
        if let Some((_sub, domain)) = hostname.split_once('.') {
            if domain.contains('.') {
                params.subject_alt_names.push(SanType::DnsName(format!("*.{domain}")));
            }
        }
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after  = rcgen::date_time_ymd(2026, 12, 31);

        let cert     = Certificate::from_params(params)?;
        let cert_pem = cert.serialize_pem_with_signer(&self.ca_cert)?;
        let key_pem  = cert.serialize_private_key_pem();

        let cert_der = pem_to_der(&cert_pem)?;
        let ca_der   = pem_to_der(&self.ca_cert.serialize_pem()?)?;
        let key_der  = pem_to_der_key(&key_pem)?;

        Ok(CachedCert {
            cert_chain: vec![RustlsCert(cert_der), RustlsCert(ca_der)],
            private_key: PrivateKey(key_der),
        })
    }

    pub fn ca_cert_path(&self) -> PathBuf {
        self.cert_dir.join("nodadark-ca.crt")
    }
}

fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
    rustls_pemfile::certs(&mut pem.as_bytes())
        .context("Parsing PEM cert")?
        .into_iter()
        .next()
        .context("Aucun certificat trouvé dans le PEM")
}

fn pem_to_der_key(pem: &str) -> Result<Vec<u8>> {
    // Fix E0597: utiliser .remove(0) au lieu de .drain().next()
    // pour éviter le borrow sur valeur droppée

    // Essayer PKCS8 d'abord
    if let Ok(mut keys) = rustls_pemfile::pkcs8_private_keys(&mut pem.as_bytes()) {
        if !keys.is_empty() {
            return Ok(keys.remove(0));
        }
    }

    // Fallback RSA
    let mut keys = rustls_pemfile::rsa_private_keys(&mut pem.as_bytes())
        .context("Parsing clé privée RSA")?;

    if keys.is_empty() {
        anyhow::bail!("Aucune clé privée trouvée dans le PEM");
    }

    Ok(keys.remove(0))
}
