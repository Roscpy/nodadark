# ⬡ NodaDark

> **Proxy d'Interception Réseau Haute Performance**  
> Un seul moteur Rust. Deux visages : Terminal et Bureau natif.

```
  ╔═╗╔╗╔╔═╗╔╦╗╔═╗╔╦╗╔═╗╦═╗╦╔═
  ║  ║║║║ ║ ║║╠═╣ ║║╠═╣╠╦╝╠╩╗
  ╚═╝╝╚╝╚═╝═╩╝╩ ╩═╩╝╩ ╩╩╚═╩ ╩
  Proxy d'Interception Réseau v0.1.0
```

---

## Table des Matières

1. [C'est quoi NodaDark ?](#cest-quoi-nodadark)
2. [Architecture](#architecture)
3. [Installation](#installation)
   - [Prérequis](#prérequis)
   - [Compiler depuis les sources](#compiler-depuis-les-sources)
   - [Termux (Android)](#termux-android)
4. [Configuration du Certificat CA](#configuration-du-certificat-ca)
   - [Android](#android)
   - [iOS](#ios)
   - [Windows](#windows)
   - [macOS / Linux](#macos--linux)
5. [Utilisation — Mode Terminal (TUI)](#utilisation--mode-terminal-tui)
   - [Lancement](#lancement)
   - [Raccourcis clavier](#raccourcis-clavier)
   - [Filtrage](#filtrage)
6. [Utilisation — Mode Bureau (Desktop)](#utilisation--mode-bureau-desktop)
7. [Utilisation — Mode Ligne de Commande](#utilisation--mode-ligne-de-commande)
8. [Règles Persistantes](#règles-persistantes)
9. [API de Contrôle](#api-de-contrôle)
10. [Sessions et Export HAR](#sessions-et-export-har)
11. [FAQ](#faq)
12. [Avertissement Légal](#avertissement-légal)

---

## C'est quoi NodaDark ?

NodaDark est un **proxy d'interception HTTP/HTTPS** (de type MITM — Man In The Middle) écrit entièrement en Rust. Il te permet de :

- **Voir** tout le trafic réseau qui passe par ton appareil ou une application cible.
- **Modifier** les requêtes (headers, cookies, body) avant qu'elles partent.
- **Rejouer** n'importe quelle requête avec ou sans modifications.
- **Bloquer** (dropper) des requêtes selon des règles que tu définis.

C'est l'alternative légère et rapide à Burp Suite ou Charles Proxy, qui tourne aussi bien sur un Raspberry Pi sans écran que sur ton laptop ou ton téléphone Android via Termux.

**Le principe fondateur : "One Core, Many Faces"**

```
[ nodadark-engine ]  ← Moteur Rust (le cerveau)
        │
        ├──▶ nodadark-tui      ← Interface Terminal (SSH, Termux, serveurs)
        └──▶ nodadark-desktop  ← Interface Bureau (Windows, macOS, Linux)
```

---

## Architecture

```
nodadark/
├── Cargo.toml                    ← Workspace Rust
├── README.md
└── crates/
    ├── nodadark-engine/          ← 🧠 Moteur (lib + binaire CLI)
    │   └── src/
    │       ├── proxy/            ← Serveur MITM, TLS, gestion des connexions
    │       ├── rules/            ← Moteur de règles (TOML)
    │       ├── storage/          ← Sauvegarde sessions, export HAR
    │       └── api/              ← Socket Unix + TCP (JSON-lines)
    │
    ├── nodadark-tui/             ← 🖥 Interface Terminal (Ratatui)
    │
    └── nodadark-desktop/         ← 🎨 Interface Bureau (Tauri + Svelte)
```

---

## Installation

### Prérequis

- **Rust** 1.75+ avec Cargo : https://rustup.rs
- **Pour le Desktop uniquement** : Node.js 18+, npm ou pnpm

```bash
# Installer Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Vérifier
rustc --version   # rustc 1.75.0 ou plus récent
cargo --version
```

---

### Compiler depuis les sources

```bash
# 1. Cloner le projet
git clone https://github.com/ton-user/nodadark.git
cd nodadark

# 2. Compiler le moteur + le TUI (en une commande)
cargo build --release

# 3. Les binaires compilés se trouvent ici :
ls target/release/
# nodadark       ← Moteur seul (mode CLI/log)
# nodadark-tui   ← Interface Terminal
```

Pour le Desktop :
```bash
cd crates/nodadark-desktop
npm install
npm run tauri build
# L'installateur (.exe / .AppImage / .dmg) se trouve dans :
# src-tauri/target/release/bundle/
```

---

### Termux (Android)

```bash
# 1. Installer les dépendances dans Termux
pkg update && pkg upgrade
pkg install rust binutils openssl-dev pkg-config

# 2. Cloner et compiler (TUI uniquement, pas de Desktop sans bureau graphique)
git clone https://github.com/ton-user/nodadark.git
cd nodadark
cargo build --release -p nodadark-tui

# 3. Optionnel : ajouter au PATH
cp target/release/nodadark-tui $PREFIX/bin/

# 4. Lancer
nodadark-tui --embedded 8080
```

> **Note Termux** : Le flag `--embedded 8080` démarre le moteur proxy intégré directement depuis le TUI sans avoir à lancer un processus séparé.

---

## Configuration du Certificat CA

Pour intercepter le trafic **HTTPS**, NodaDark génère un certificat CA racine auto-signé. Tu dois l'installer comme autorité de confiance sur l'appareil dont tu veux analyser le trafic.

Le certificat est généré automatiquement au premier lancement et sauvegardé ici :
- **Linux / Android** : `~/.config/nodadark/certs/nodadark-ca.crt`
- **Windows** : `%APPDATA%\nodadark\certs\nodadark-ca.crt`
- **macOS** : `~/Library/Application Support/nodadark/certs/nodadark-ca.crt`

---

### Android

1. Copie `nodadark-ca.crt` sur le téléphone (via ADB ou partage de fichier).
2. Va dans **Paramètres → Sécurité → Chiffrement et identifiants → Installer depuis la mémoire**.
3. Sélectionne le fichier `.crt`.
4. Donne-lui un nom (ex: "NodaDark CA").
5. Choisis **VPN et applications** ou **Wi-Fi** selon ton besoin.

---

### iOS

1. Envoie le fichier `nodadark-ca.crt` sur l'iPhone (mail, AirDrop, etc.).
2. Ouvre le fichier → **Installer le profil**.
3. Va dans **Réglages → Profil téléchargé → Installer**.
4. Ensuite : **Réglages → Général → À propos → Réglages du certificat** → Active la confiance totale pour le certificat NodaDark.

---

### Windows

1. Double-clique sur `nodadark-ca.crt`.
2. Clique sur **Installer le certificat**.
3. Choisis **Ordinateur local** → **Suivant**.
4. Sélectionne **Placer tous les certificats dans le magasin suivant**.
5. Parcourir → **Autorités de certification racines de confiance** → OK → Terminer.

---

### macOS / Linux

```bash
# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain \
  ~/.config/nodadark/certs/nodadark-ca.crt

# Ubuntu / Debian
sudo cp ~/.config/nodadark/certs/nodadark-ca.crt /usr/local/share/ca-certificates/nodadark.crt
sudo update-ca-certificates

# Arch / Fedora
sudo trust anchor --store ~/.config/nodadark/certs/nodadark-ca.crt
```

---

## Utilisation — Mode Terminal (TUI)

### Lancement

```bash
# Option 1 : TUI seul (le moteur doit déjà tourner séparément)
nodadark-tui --socket /tmp/nodadark.sock

# Option 2 : TUI avec moteur intégré (tout-en-un, recommandé)
nodadark-tui --embedded 8080

# Option 3 : Via TCP si le socket Unix n'est pas disponible (Windows)
nodadark-tui --port 9090

# Aide complète
nodadark-tui --help
```

### Interface

```
┌─────────────────────────────────────────────────────────────────┐
│ ⬡ NodaDark v0.1  ▶ LIVE  🔒 MITM  ● Proxy :8080  142 requêtes │
│         [q]Quit [p]Pause [/]Filtre [j/k]Nav [Enter]Détail      │
└──────────────────────────────────┬──────────────────────────────┘
│ Requêtes (142)                   │ [1]Headers [2]Body [3]Hex   │
│ 🔒[GET   ] 200 api.example.com   │ ────── REQUEST HEADERS ──── │
│ 🔒[POST  ] 302 login.target.com  │ Host: api.example.com       │
│ ▶🔒[GET  ] 500 vuln.site/xss    │ Cookie: session=abc123…     │
│                                  │ Authorization: Bearer tok…  │
│                                  │ ─── RESPONSE HEADERS ───── │
│                                  │ Content-Type: application/  │
│                                  │ json                        │
└──────────────────────────────────┴─────────────────────────────┘
│ [⏸ PAUSE] | Filtre: *.example.com | ✓ Connecté au moteur      │
└─────────────────────────────────────────────────────────────────┘
```

### Raccourcis clavier

| Touche | Action |
|--------|--------|
| `j` / `↓` | Descendre dans la liste |
| `k` / `↑` | Monter dans la liste |
| `G` | Aller à la dernière requête |
| `g` | Aller à la première requête |
| `PageDown` | Descendre de 10 lignes |
| `PageUp` | Monter de 10 lignes |
| `Enter` | Ouvrir le détail de la requête |
| `Tab` | Basculer entre onglets Headers / Body / Hex |
| `1` `2` `3` | Aller directement à l'onglet (dans le détail) |
| `a` | Ouvrir le menu d'actions (Replay, Edit, Drop…) |
| `r` | Rejouer la requête sélectionnée |
| `d` | Dropper la requête sélectionnée |
| `i` | Charger le détail complet depuis le moteur |
| `e` | Ouvrir l'éditeur de cookies |
| `p` | Basculer Pause / Reprise du proxy |
| `/` | Activer le filtre de recherche |
| `Ctrl+C` | Effacer tout l'historique |
| `Esc` | Retour à la liste / fermer popup |
| `q` | Quitter |

### Filtrage

Appuie sur `/` pour ouvrir la barre de recherche. Le filtrage est **live** (instantané) et cherche dans :
- L'URL complète
- Le nom de domaine (host)
- La méthode HTTP (`GET`, `POST`…)
- Le code de statut (`200`, `404`…)

Exemples :
```
/google.com        ← Toutes les requêtes vers google.com
/POST              ← Uniquement les POST
/500               ← Uniquement les erreurs 500
/api               ← Toutes les URLs contenant "api"
```

Appuie sur `Enter` pour valider, `Esc` pour effacer le filtre.

---

## Utilisation — Mode Bureau (Desktop)

Lance l'application `.exe` (Windows), `.app` (macOS), ou `.AppImage` (Linux).

**Premier lancement :**
1. Le proxy démarre automatiquement sur le port **8080**.
2. Configure ton appareil/navigateur pour utiliser `127.0.0.1:8080` comme proxy HTTP/HTTPS.
3. Installe le certificat CA affiché dans ⚙ **Paramètres**.

**Fonctionnalités de l'interface :**

| Zone | Description |
|------|-------------|
| Toolbar | Démarrer/Arrêter, Pause, filtre de scope, effacer |
| Liste (gauche) | Flux live des requêtes avec codes couleur |
| Détail (droite) | Headers, Body formaté JSON, Hex viewer |
| 🍪 Cookie Editor | Éditer les cookies et renvoyer la requête |

**Raccourcis clavier Desktop :**

| Raccourci | Action |
|-----------|--------|
| `Ctrl+P` | Pause / Reprise |
| `Ctrl+L` | Focus sur le filtre de recherche |
| `Ctrl+R` | Rejouer la requête sélectionnée |
| `Esc` | Désélectionner / fermer modal |
| `1` `2` `3` | Changer d'onglet dans le détail |

---

## Utilisation — Mode Ligne de Commande

Pour une utilisation sur serveur sans interface :

```bash
# Démarrage basique
nodadark --port 8080

# Avec logs détaillés
nodadark --port 8080 --verbose

# Mode strict (bloque les certificats invalides)
nodadark --port 8080 --strict

# Changer le port de l'API de contrôle
nodadark --port 8080 --api-port 9090

# Voir toutes les options
nodadark --help
```

Le proxy loggue tout le trafic dans le terminal. Configure ton appareil pour passer par `IP_SERVEUR:8080`.

---

## Règles Persistantes

Les règles permettent d'automatiser des actions sur le trafic. Elles sont dans :
- **Linux / Android** : `~/.config/nodadark/rules.toml`
- **Windows** : `%APPDATA%\nodadark\rules.toml`

Format TOML :

```toml
# Exemple 1 : Supprimer un header sur un domaine spécifique
[[rules]]
name = "Supprimer X-Frame-Options sur example.com"
enabled = true
domain = "*.example.com"
action = { type = "remove_header", name = "X-Frame-Options" }

# Exemple 2 : Remplacer le User-Agent sur tous les sites
[[rules]]
name = "Fake User-Agent"
enabled = true
action = { type = "modify_header", name = "User-Agent", value = "Mozilla/5.0 NodaDark" }

# Exemple 3 : Dropper toutes les requêtes vers un tracker
[[rules]]
name = "Bloquer tracker analytics"
enabled = true
domain = "*.analytics-tracker.com"
action = { type = "drop" }

# Exemple 4 : Injecter un header sur une API spécifique
[[rules]]
name = "Ajouter X-Debug sur /api/*"
enabled = true
domain = "api.monapp.com"
path = "/api/*"
action = { type = "inject_header", name = "X-Debug-Mode", value = "true" }
```

**Champs disponibles :**

| Champ | Description | Exemple |
|-------|-------------|---------|
| `name` | Nom lisible de la règle | `"Bloquer pub"` |
| `enabled` | Activer/désactiver sans supprimer | `true` / `false` |
| `domain` | Filtre de domaine (glob `*` supporté) | `"*.google.com"` |
| `path` | Filtre de chemin | `"/api/*"` |
| `method` | Filtre de méthode HTTP | `"POST"` |
| `action.type` | Action : `drop`, `modify_header`, `remove_header`, `inject_header` | |

Les règles sont lues au démarrage du moteur. Pour les recharger sans redémarrer : relance simplement le binaire.

---

## API de Contrôle

NodaDark expose une API JSON-lines sur deux interfaces :

- **Socket Unix** : `/tmp/nodadark.sock` (Linux / macOS / Android)
- **TCP** : `127.0.0.1:9090`

### Connexion

```bash
# Via netcat (TCP)
nc 127.0.0.1 9090

# Via socat (Unix socket)
socat - UNIX-CONNECT:/tmp/nodadark.sock
```

### Commandes disponibles

```json
// Mettre en pause
{"command":"pause"}

// Reprendre
{"command":"resume"}

// Lister les requêtes (paginées, avec filtre optionnel)
{"command":"list_requests","offset":0,"limit":50,"filter":"google"}

// Obtenir le détail d'une requête
{"command":"get_request","id":"abc123-..."}

// Dropper une requête
{"command":"drop","id":"abc123-..."}

// Rejouer une requête (avec headers modifiés optionnels)
{"command":"replay","id":"abc123-...","modified_headers":{"Cookie":"session=NEW_VALUE"}}

// Effacer tout l'historique
{"command":"clear_requests"}

// Sauvegarder la session
{"command":"save_session","name":"audit-2024"}

// Exporter en HAR
{"command":"export_har","name":"export"}

// État du proxy
{"command":"status"}

// S'abonner aux événements temps réel (streaming)
{"command":"subscribe"}
```

### Réponses

```json
// Succès
{"type":"ok","message":"Proxy mis en pause"}

// Erreur
{"type":"error","message":"Requête introuvable"}

// État
{"type":"status","paused":false,"port":8080,"request_count":142,"ca_path":"/home/user/.config/nodadark/certs/nodadark-ca.crt"}

// Liste de requêtes
{"type":"requests","items":[...],"total":142}

// Événement temps réel (après subscribe)
{"type":"request","id":"abc","method":"GET","url":"https://...","tls":true,"timestamp":"..."}
{"type":"response","id":"abc","status":200,"duration_ms":45,"size":1024}
```

### Exemple : script bash d'automatisation

```bash
#!/bin/bash
# Mettre le proxy en pause, attendre 5 secondes, reprendre

echo '{"command":"pause"}' | nc -q1 127.0.0.1 9090
echo "Proxy en pause..."
sleep 5
echo '{"command":"resume"}' | nc -q1 127.0.0.1 9090
echo "Proxy repris."
```

---

## Sessions et Export HAR

### Sauvegarder une session

```bash
# Via l'API
echo '{"command":"save_session","name":"test-login"}' | nc -q1 127.0.0.1 9090

# Les sessions sont sauvegardées dans :
# Linux : ~/.local/share/nodadark/sessions/test-login-20240101-120000.nds
# Windows : %LOCALAPPDATA%\nodadark\sessions\
```

### Exporter en HAR

Le format HAR (HTTP Archive) est compatible avec :
- **Chrome DevTools** (onglet Réseau → Import)
- **Burp Suite** (Import HAR)
- **Analyse de performance** en ligne (har.tech, etc.)

```bash
echo '{"command":"export_har","name":"audit"}' | nc -q1 127.0.0.1 9090
```

---

## FAQ

**Q : Comment configurer mon téléphone pour passer par NodaDark ?**  
R : Va dans les paramètres Wi-Fi de ton téléphone → maintiens appuyé sur le réseau → Modifier → Proxy Manuel → Hostname : `IP_DE_TON_PC` → Port : `8080`. Puis installe le certificat CA.

**Q : NodaDark tourne-t-il sur Windows ?**  
R : Oui. Le socket Unix n'est pas disponible sur Windows mais le TCP (port 9090) fonctionne. Le TUI tourne dans Windows Terminal ou PowerShell. Le Desktop Tauri génère un `.exe` natif.

**Q : Est-ce que ça marche avec les applis Android qui épinglent leurs certificats (certificate pinning) ?**  
R : Les applis avec pinning ne font pas confiance à un CA externe. Pour les contourner, il faut patcher l'APK (via Frida ou Objection) ou utiliser un émulateur rooté. NodaDark gère la partie proxy, mais le contournement du pinning est une étape séparée.

**Q : Quelle différence avec Burp Suite ?**  
R : Burp nécessite Java et pèse plusieurs centaines de Mo. NodaDark est un binaire Rust statique de quelques Mo, sans dépendance. Il tourne sur ARM (Termux, Raspberry Pi). En contrepartie, Burp Suite a des fonctionnalités d'audit plus avancées (scanner de vulnérabilités, Intruder, etc.).

**Q : Le moteur peut-il tourner en arrière-plan comme un daemon ?**  
R : Oui, lance `nodadark --port 8080` dans un `tmux` ou `screen`, ou crée un service systemd. Exemple :

```ini
# /etc/systemd/system/nodadark.service
[Unit]
Description=NodaDark Proxy
After=network.target

[Service]
ExecStart=/usr/local/bin/nodadark --port 8080 --bind 0.0.0.0
Restart=on-failure
User=ton-user

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable nodadark
sudo systemctl start nodadark
```

**Q : Le TUI ne se connecte pas au moteur.**  
R : Vérifie que le moteur tourne (`nodadark --port 8080`) et que le socket existe (`ls /tmp/nodadark.sock`). Si tu es sur Windows, utilise `--port 9090` à la place du socket Unix.

---

## Avertissement Légal

> ⚠ **NodaDark est un outil d'audit de sécurité réseau.**  
> Son utilisation est **strictement réservée** aux réseaux et appareils sur lesquels tu as une autorisation explicite.  
> Intercepter le trafic réseau sans autorisation est **illégal** dans la plupart des pays.  
> L'auteur décline toute responsabilité pour toute utilisation abusive de cet outil.  
> **Utilise-le uniquement dans un cadre légal : test de pénétration autorisé, débogage de tes propres applications, audit de sécurité avec accord écrit.**

---

*NodaDark — "One Core, Many Faces" — Fait avec ❤ en Rust*
# nodadark
# nodadark
