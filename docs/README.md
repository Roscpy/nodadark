# ⬡ NodaDark

> **Proxy d'Interception Réseau Haute Performance**  
> Un seul moteur Rust. Deux visages : Terminal et Bureau natif.

```
  ╔═╗╔╗╔╔═╗╔╦╗╔═╗╔╦╗╔═╗╦═╗╦╔═
  ║  ║║║║ ║ ║║╠═╣ ║║╠═╣╠╦╝╠╩╗
  ╚═╝╝╚╝╚═╝═╩╝╩ ╩═╩╝╩ ╩╩╚═╩ ╩
  Proxy d'Interception Réseau v0.1.0
  ⚠  À utiliser uniquement sur des réseaux autorisés.
```

---

## Table des Matières

1. [C'est quoi NodaDark ?](#cest-quoi-nodadark-)
2. [Architecture](#architecture)
3. [Installation sur Termux (Android)](#installation-sur-termux-android)
4. [Lancement — Méthode Officielle (2 Sessions Termux)](#lancement--méthode-officielle-2-sessions-termux)
5. [Configuration du Certificat CA](#configuration-du-certificat-ca)
6. [Générer du trafic et voir les résultats](#générer-du-trafic-et-voir-les-résultats)
7. [Interface TUI — Tous les raccourcis](#interface-tui--tous-les-raccourcis)
8. [Règles Persistantes](#règles-persistantes)
9. [API de Contrôle](#api-de-contrôle)
10. [Sessions et Export HAR](#sessions-et-export-har)
11. [FAQ](#faq)
12. [Avertissement Légal](#avertissement-légal)

---

## C'est quoi NodaDark ?

NodaDark est un **proxy d'interception HTTP/HTTPS** (MITM — Man In The Middle) écrit
entièrement en Rust. Il te permet de :

- **Voir** tout le trafic réseau qui passe par ton appareil ou une application cible
- **Modifier** les requêtes (headers, cookies, body) avant qu'elles partent
- **Rejouer** n'importe quelle requête avec ou sans modifications
- **Bloquer** des requêtes selon des règles que tu définis

C'est l'alternative légère et rapide à Burp Suite ou Charles Proxy,
qui tourne aussi bien sur un Samsung A15 sous Termux que sur un serveur Linux.

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
├── Cargo.toml
└── crates/
    ├── nodadark-engine/   ← 🧠 Moteur (proxy MITM, TLS, règles, API)
    ├── nodadark-tui/      ← 🖥  Interface Terminal (Ratatui)
    └── nodadark-desktop/  ← 🎨 Interface Bureau (Tauri + Svelte)
```

---

## Installation sur Termux (Android)

```bash
# 1. Mettre à jour Termux
pkg update && pkg upgrade -y

# 2. Installer les dépendances
pkg install rust binutils openssl-dev pkg-config git -y

# 3. Cloner le projet
git clone https://github.com/roscpy/nodadark.git
cd nodadark

# 4. Compiler le moteur
cargo build --release -p nodadark-engine

# 5. Compiler l'interface TUI
cargo build --release -p nodadark-tui

# 6. Vérifier que les binaires existent
ls ~/nodadark/target/release/nodadark*
# nodadark       ← moteur
# nodadark-tui   ← interface terminal
```

> **Note :** La compilation prend environ 1 à 2 minutes sur un bon téléphone Android.
> Une fois compilé, le binaire fonctionne sans recompiler.

---

## Lancement — Méthode Officielle (2 Sessions Termux)

> ⚠️ NodaDark nécessite **2 sessions Termux séparées** pour fonctionner correctement.
> Le moteur tourne en arrière-plan dans la Session 1, et le TUI se connecte dans la Session 2.

### Ouvrir 2 sessions dans Termux

```
Swipe depuis la gauche de l'écran → "New Session"
```

---

### Session 1 — Lancer le moteur (en arrière-plan)

```bash
cd ~/nodadark

# Lancer le moteur sur le port 8080 en arrière-plan
./target/release/nodadark --port 8080 &

# Vérifier qu'il tourne
sleep 2
ps aux | grep nodadark | grep -v grep
```

Tu dois voir quelque chose comme :
```
u0_a344  12345  0.0  0.1  nodadark --port 8080
```

**Sortie attendue au démarrage :**
```
  ╔═╗╔╗╔╔═╗╔╦╗╔═╗╔╦╗╔═╗╦═╗╦╔═
  ...
  INFO 🚀 Proxy démarré sur 127.0.0.1:8080
  INFO 🔌 API TCP : 127.0.0.1:9090
  INFO 🔒 NodaDark CA prêt : ~/.config/nodadark/certs/nodadark-ca.crt
  INFO ✅ 2 règle(s) chargée(s)
```

---

### Session 2 — Lancer le TUI (interface)

Ouvre une nouvelle session Termux (swipe gauche → New Session), puis :

```bash
cd ~/nodadark
./target/release/nodadark-tui --port 9090
```

**Interface attendue :**
```
┌─────────────────────────────────────────────────────────────┐
│ ⬡ NodaDark v0.1  ▶ LIVE  🔒 MITM  ● Proxy :8080           │
├─────────────────────────────────────────────────────────────┤
│ Requetes (0)                                                │
│                                                             │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ Connecte -- Proxy :8080                                     │
└─────────────────────────────────────────────────────────────┘
```

> ✅ Si tu vois **● (point vert)** à côté de "Proxy :8080" → le TUI est bien connecté au moteur.  
> ❌ Si tu vois **○ (cercle vide)** → le moteur n'est pas lancé, retourne en Session 1.

---

### Session 3 — Générer du trafic (pour tester)

Ouvre une 3ème session et envoie des requêtes via NodaDark :

```bash
# Tester avec httpbin.org (site de test HTTP)
curl --proxy http://127.0.0.1:8080 \
  --cacert ~/.config/nodadark/certs/nodadark-ca.crt \
  -s https://httpbin.org/get -o /dev/null -w "%{http_code}"
```

Retourne sur la **Session 2 (TUI)** — tu verras la requête apparaître :
```
Requetes (1)
> [S][GET] 200 httpbin.org/get  1460ms
```

---

## Configuration du Certificat CA

Pour intercepter le trafic **HTTPS**, NodaDark génère automatiquement un certificat CA
racine au premier lancement. Tu dois l'installer sur l'appareil cible.

**Chemin du certificat :**
```bash
~/.config/nodadark/certs/nodadark-ca.crt
# ou sur Termux :
/data/data/com.termux/files/home/.config/nodadark/certs/nodadark-ca.crt
```

### Copier le CA vers le stockage Android

```bash
cp ~/.config/nodadark/certs/nodadark-ca.crt /sdcard/Download/nodadark-ca.crt
```

### Installer dans Firefox Android (recommandé — fonctionne sans root)

```
1. Ouvre Firefox Android
2. Menu (3 points) → Paramètres
3. Sécurité et confidentialité
4. Certificats → Importer un certificat
5. Sélectionne /sdcard/Download/nodadark-ca.crt
```

### Installer dans Firefox via about:config (proxy sans Wi-Fi, 4G OK)

```
1. Dans Firefox, tape : about:config
2. network.proxy.type → 1
3. network.proxy.http → 127.0.0.1
4. network.proxy.http_port → 8080
5. network.proxy.ssl → 127.0.0.1
6. network.proxy.ssl_port → 8080
```

### Sur Android (système) — si Wi-Fi disponible

```
Paramètres → Sécurité → Installer depuis la mémoire →
Sélectionne nodadark-ca.crt → "NodaDark CA"

Wi-Fi → Maintenir appuyé sur le réseau → Modifier →
Proxy Manuel → 127.0.0.1 → Port 8080
```

### Sur Linux / macOS

```bash
# Ubuntu/Debian
sudo cp ~/.config/nodadark/certs/nodadark-ca.crt \
  /usr/local/share/ca-certificates/nodadark.crt
sudo update-ca-certificates

# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain \
  ~/.config/nodadark/certs/nodadark-ca.crt
```

### Via curl (sans installation CA système)

```bash
# Spécifier le CA directement dans curl
curl --proxy http://127.0.0.1:8080 \
  --cacert ~/.config/nodadark/certs/nodadark-ca.crt \
  https://cible.com
```

---

## Générer du trafic et voir les résultats

### Test de base — httpbin.org

```bash
# Session 3 Termux
curl --proxy http://127.0.0.1:8080 \
  --cacert ~/.config/nodadark/certs/nodadark-ca.crt \
  -s https://httpbin.org/get
```

**Ce que tu verras dans le TUI :**
```
[S][GET] 200 httpbin.org/get  1460ms
```

**Onglet 2 — Body :**
```json
{
  "args": {},
  "headers": {
    "Accept": "*/*",
    "Host": "httpbin.org",
    "User-Agent": "curl/8.20.0"
  },
  "origin": "TON_IP",
  "url": "https://httpbin.org/get"
}
```

### Test POST — voir le body envoyé

```bash
curl --proxy http://127.0.0.1:8080 \
  --cacert ~/.config/nodadark/certs/nodadark-ca.crt \
  -X POST https://httpbin.org/post \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"secret123"}'
```

### Test avec plusieurs requêtes simultanées

```bash
for site in httpbin.org/get httpbin.org/post httpbin.org/headers; do
  curl --proxy http://127.0.0.1:8080 \
    --cacert ~/.config/nodadark/certs/nodadark-ca.crt \
    -s https://$site -o /dev/null &
done
wait
```

---

## Interface TUI — Tous les raccourcis

### Navigation

| Touche | Action |
|--------|--------|
| `j` / `↓` | Descendre dans la liste |
| `k` / `↑` | Monter dans la liste |
| `G` | Aller à la dernière requête |
| `g` | Aller à la première requête |
| `PageDown` | Descendre de 10 lignes |
| `PageUp` | Monter de 10 lignes |

### Sélection et Détail

| Touche | Action |
|--------|--------|
| `Enter` | Ouvrir le détail de la requête |
| `Esc` | Retour à la liste / fermer popup |
| `Tab` | Basculer Headers → Body → Hex |
| `1` | Onglet Headers |
| `2` | Onglet Body (JSON formaté automatiquement) |
| `3` | Onglet Hex Viewer |

### Actions

| Touche | Action |
|--------|--------|
| `a` | Menu d'actions (Replay, Edit, Drop...) |
| `r` | Rejouer la requête directement |
| `d` | Dropper la requête |
| `e` | Éditeur de cookies |
| `p` | Pause / Reprise du proxy |
| `i` | Charger le détail complet |
| `Ctrl+C` | Effacer tout l'historique |
| `q` | Quitter |

### Filtrage

| Touche | Action |
|--------|--------|
| `/` | Activer la recherche live |
| `/google` | Filtrer par domaine |
| `/POST` | Filtrer par méthode |
| `/500` | Filtrer par code d'erreur |
| `Enter` | Valider le filtre |
| `Esc` | Effacer le filtre |

### Légende des couleurs

| Couleur | Signification |
|---------|---------------|
| 🟢 Vert | Code 2xx — Succès |
| 🟡 Jaune | Code 3xx — Redirection |
| 🔴 Rouge | Code 4xx/5xx — Erreur |
| 🔵 Cyan | Requête en attente |
| ⬛ Gris | Requête droppée |
| 🔒 [S] | Requête HTTPS (SSL) |

---

## Règles Persistantes

Fichier : `~/.config/nodadark/rules.toml`

```toml
# Bloquer un tracker
[[rules]]
name = "Bloquer analytics"
enabled = true
domain = "*.analytics.com"
action = { type = "drop" }

# Modifier le User-Agent
[[rules]]
name = "Fake User-Agent"
enabled = true
action = { type = "modify_header", name = "User-Agent", value = "NodaDark Audit" }

# Supprimer un header
[[rules]]
name = "Supprimer X-Frame-Options"
enabled = true
domain = "*.example.com"
action = { type = "remove_header", name = "X-Frame-Options" }

# Injecter un header de debug
[[rules]]
name = "Mode debug API"
enabled = true
domain = "api.monapp.com"
path = "/api/*"
action = { type = "inject_header", name = "X-Debug-Mode", value = "true" }
```

Les règles sont lues au démarrage. Pour les recharger, redémarre le moteur.

---

## API de Contrôle

NodaDark expose une API JSON-lines sur :
- **TCP** : `127.0.0.1:9090`
- **Socket Unix** : `/tmp/nodadark.sock` (Linux/Android)

```bash
# Vérifier l'état
echo '{"command":"status"}' | nc -q1 127.0.0.1 9090

# Mettre en pause
echo '{"command":"pause"}' | nc -q1 127.0.0.1 9090

# Reprendre
echo '{"command":"resume"}' | nc -q1 127.0.0.1 9090

# Lister les 10 dernières requêtes
echo '{"command":"list_requests","limit":10}' | nc -q1 127.0.0.1 9090

# Rejouer une requête
echo '{"command":"replay","id":"ID_ICI"}' | nc -q1 127.0.0.1 9090

# Dropper une requête
echo '{"command":"drop","id":"ID_ICI"}' | nc -q1 127.0.0.1 9090

# Effacer l'historique
echo '{"command":"clear_requests"}' | nc -q1 127.0.0.1 9090

# S'abonner aux événements live (streaming)
echo '{"command":"subscribe"}' | nc 127.0.0.1 9090
```

---

## Sessions et Export HAR

```bash
# Sauvegarder la session actuelle
echo '{"command":"save_session","name":"audit-client"}' | nc -q1 127.0.0.1 9090

# Exporter en HAR (compatible Chrome DevTools, Burp)
echo '{"command":"export_har","name":"export"}' | nc -q1 127.0.0.1 9090

# Fichiers sauvegardés dans :
# ~/.local/share/nodadark/sessions/
```

---

## FAQ

**Q : Le TUI affiche "○ Proxy :8080" (cercle vide) — que faire ?**
R : Le moteur n'est pas lancé. Va en Session 1 et vérifie :
```bash
ps aux | grep nodadark | grep -v grep
# Si vide → relancer : cd ~/nodadark && ./target/release/nodadark --port 8080 &
```

**Q : curl retourne "proxychains: can't load process"**
R : Proxychains intercepte curl. Solution :
```bash
# Vérifier si curl est une fonction dans .bashrc
grep curl ~/.bashrc
# Si oui, supprimer la fonction et recharger :
sed -i '/proxychains/d' ~/.bashrc && source ~/.bashrc
```

**Q : Comment tester sans Wi-Fi (4G seulement) ?**
R : Utilise curl avec `--proxy` directement — ça fonctionne sur 4G :
```bash
curl --proxy http://127.0.0.1:8080 \
  --cacert ~/.config/nodadark/certs/nodadark-ca.crt \
  https://cible.com
```

**Q : Le moteur s'arrête immédiatement après le lancement ?**
R : Utilise `&` pour le lancer en arrière-plan :
```bash
./target/release/nodadark --port 8080 &
```

**Q : Comment intercepter un autre appareil ?**
R : Active le hotspot sur ton téléphone, lance NodaDark avec `--bind 0.0.0.0` :
```bash
./target/release/nodadark --port 8080 --bind 0.0.0.0 &
```
L'autre appareil utilise ton IP comme proxy.

**Q : Certificat pinning — NodaDark ne voit pas le trafic d'une app ?**
R : L'app utilise du certificate pinning. Sans root + Frida, ce trafic est inaccessible. NodaDark intercepte les apps sans pinning et les navigateurs.

---

## Avertissement Légal

> ⚠️ **NodaDark est un outil d'audit de sécurité réseau.**  
> Son utilisation est **strictement réservée** aux réseaux et appareils pour lesquels  
> tu as une **autorisation explicite et écrite**.  
> Intercepter le trafic réseau sans autorisation est **illégal** dans la plupart des pays.  
> L'auteur décline toute responsabilité pour toute utilisation abusive.  
> **Usage légal uniquement : pentest autorisé, débogage de tes propres apps, audit avec accord écrit.**

---

*NodaDark v0.1.0 — "One Core, Many Faces" — Fait avec ❤ en Rust sur Samsung A15 / Termux*

---

## 📸 Screenshots — NodaDark en Action (Tests Réels)

> Toutes les captures ci-dessous ont été prises sur un **Samsung A15 Android**  
> avec **Termux**, en **4G** (sans Wi-Fi), le **03 Mai 2026**.

---

### 1. Liste live — Interception de google.com, github.com et cloudflare.com

![NodaDark TUI - Liste requêtes Google GitHub Cloudflare](docs/screenshots/screenshot_1_liste_google_github_cloudflare.jpg)

```
Requetes (3)
> [S][GET]      ... google.com          ← En attente (cyan)
  [S][GET]  200 github.com      405ms  ← Succès (vert)
  [S][GET]  301 cloudflare.com  406ms  ← Redirection (jaune)
```

**Ce qu'on voit :**
- `[S]` = Requête HTTPS (SSL intercepté par NodaDark)
- `[GET]` = Méthode HTTP
- `200` = Réponse GitHub en vert ✅
- `301` = Redirection Cloudflare en jaune 🟡
- `...` = Google encore en attente (cyan) 🔵
- `● Proxy :8080` = Point vert → TUI connecté au moteur

---

### 2. Onglet Headers — Request & Response

![NodaDark TUI - Headers httpbin.org](docs/screenshots/screenshot_2_headers.jpg)

```
GET HTTPS https://httpbin.org:443/get  →  200 (1460ms)

---- REQUEST HEADERS ----
host:        httpbin.org
user-agent:  curl/8.20.0
accept:      */*

---- RESPONSE HEADERS ----
date:          Sun, 03 May 2026 21:31:12 GMT
content-type:  application/json
```

**Ce qu'on voit :**
- **REQUEST HEADERS** (en cyan) = ce que ton appareil envoie
- **RESPONSE HEADERS** (en cyan) = ce que le serveur répond
- Les headers sensibles (`Cookie`, `Authorization`) apparaissent en jaune automatiquement

---

### 3. Onglet Body — JSON formaté automatiquement

![NodaDark TUI - Body JSON httpbin.org](docs/screenshots/screenshot_3_body_json.jpg)

```
Body (255 octets)
{
    "args": {},
    "headers": {
        "Accept": "*/*",
        "Host": "httpbin.org",
        "User-Agent": "curl/8.20.0",
        "X-Amzn-Trace-Id": "Root=1-69f7bea0-..."
    },
    ...
}
```

**Ce qu'on voit :**
- Body JSON **formaté et indenté automatiquement** par NodaDark
- Taille du body affichée : `255 octets`
- En pentest : c'est ici qu'on voit les **mots de passe**, **tokens**, **données POST**

---

### 4. Onglet Hex Viewer — Données brutes

![NodaDark TUI - Hex Viewer httpbin.org](docs/screenshots/screenshot_4_hex_viewer.jpg)

```
Hex Viewer (255 octets)
00000000  7b 0a 20 20 22 61 72 67...  | {.  "arg
00000010  0a 20 20 22 68 65 61 64...  | ..  "head
00000020  20 20 20 20 22 41 63 63...  |     "Acc
00000030  2f 2a 22 2c 20 0a 20 20...  | /*",  .
```

**Ce qu'on voit :**
- Colonne gauche = **offset hexadécimal** (position dans les données)
- Colonne centrale = **valeurs hexadécimales** des octets (cyan)
- Colonne droite = **représentation ASCII** des mêmes octets
- Utile pour analyser les **données binaires**, détecter des caractères cachés

---

### Comment placer les screenshots dans le repo

```bash
# Créer le dossier dans ton repo GitHub
mkdir -p docs/screenshots

# Copier tes screenshots dedans
cp screenshot_*.jpg docs/screenshots/

# Commit
git add docs/screenshots/
git commit -m "feat: add real demo screenshots"
git push
```

