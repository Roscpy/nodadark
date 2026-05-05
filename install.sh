#!/data/data/com.termux/files/usr/bin/bash
# ═══════════════════════════════════════════════════════════
#  NodaDark — Script d'installation automatique
#  Usage : bash install.sh
#  Ce script :
#    1. Vérifie les dépendances
#    2. Compile les binaires
#    3. Les installe dans le PATH
#    4. Génère le certificat CA
#    5. Affiche le résumé final
# ═══════════════════════════════════════════════════════════

set -e  # Arrêter si une commande échoue

# ── Couleurs ────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# ── Fonctions ────────────────────────────────────────────────
ok()   { echo -e "${GREEN}✅ $1${RESET}"; }
warn() { echo -e "${YELLOW}⚠  $1${RESET}"; }
err()  { echo -e "${RED}✗  $1${RESET}"; exit 1; }
info() { echo -e "${CYAN}→  $1${RESET}"; }
step() { echo -e "\n${BOLD}${CYAN}══ $1 ══${RESET}"; }

# ── Bannière ────────────────────────────────────────────────
echo -e "${CYAN}"
cat << 'BANNER'
  ╔═╗╔╗╔╔═╗╔╦╗╔═╗╔╦╗╔═╗╦═╗╦╔═
  ║  ║║║║ ║ ║║╠═╣ ║║╠═╣╠╦╝╠╩╗
  ╚═╝╝╚╝╚═╝═╩╝╩ ╩═╩╝╩ ╩╩╚═╩ ╩
  Script d'installation v0.1.0
BANNER
echo -e "${RESET}"

# ── Variables ────────────────────────────────────────────────
INSTALL_DIR="$PREFIX/bin"
CA_DIR="$HOME/.config/nodadark/certs"
RULES_DIR="$HOME/.config/nodadark"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Détecter si on est dans le bon dossier
if [ ! -f "$SCRIPT_DIR/Cargo.toml" ]; then
    err "Lance ce script depuis la racine du projet nodadark !\ncd ~/nodadark && bash install.sh"
fi

# ═══════════════════════════════════════════════════════════
# ÉTAPE 1 — Vérification des dépendances
# ═══════════════════════════════════════════════════════════
step "ÉTAPE 1 — Vérification des dépendances"

deps_missing=0

check_dep() {
    if command -v "$1" &>/dev/null; then
        ok "$1 trouvé ($(command -v $1))"
    else
        warn "$1 manquant — installation..."
        pkg install "$2" -y 2>/dev/null || warn "Impossible d'installer $2 automatiquement"
        deps_missing=1
    fi
}

check_dep "rustc"    "rust"
check_dep "cargo"    "rust"
check_dep "pkg-config" "pkg-config"
check_dep "openssl"  "openssl"
check_dep "git"      "git"
check_dep "nc"       "netcat"

if [ $deps_missing -eq 1 ]; then
    warn "Certaines dépendances ont été installées. Relance le script si erreur."
fi

# Vérifier la version de Rust
RUST_VERSION=$(rustc --version 2>/dev/null | awk '{print $2}')
info "Rust version : $RUST_VERSION"

# ═══════════════════════════════════════════════════════════
# ÉTAPE 2 — Compilation
# ═══════════════════════════════════════════════════════════
step "ÉTAPE 2 — Compilation des binaires"

info "Compilation du moteur (nodadark-engine)..."
if cargo build --release -p nodadark-engine 2>&1; then
    ok "nodadark-engine compilé"
else
    err "Échec de la compilation de nodadark-engine"
fi

info "Compilation du TUI (nodadark-tui)..."
if cargo build --release -p nodadark-tui 2>&1; then
    ok "nodadark-tui compilé"
else
    err "Échec de la compilation de nodadark-tui"
fi

# Vérifier que les binaires existent
[ -f "$SCRIPT_DIR/target/release/nodadark" ]     || err "Binaire nodadark introuvable"
[ -f "$SCRIPT_DIR/target/release/nodadark-tui" ] || err "Binaire nodadark-tui introuvable"
ok "Binaires compilés avec succès"

# ═══════════════════════════════════════════════════════════
# ÉTAPE 3 — Installation dans le PATH
# ═══════════════════════════════════════════════════════════
step "ÉTAPE 3 — Installation dans le PATH ($INSTALL_DIR)"

cp "$SCRIPT_DIR/target/release/nodadark"     "$INSTALL_DIR/nodadark"
cp "$SCRIPT_DIR/target/release/nodadark-tui" "$INSTALL_DIR/nodadark-tui"
chmod +x "$INSTALL_DIR/nodadark"
chmod +x "$INSTALL_DIR/nodadark-tui"

# Vérifier que c'est dans le PATH
if command -v nodadark &>/dev/null; then
    ok "nodadark installé → $(which nodadark)"
else
    err "Impossible d'installer dans $INSTALL_DIR"
fi
if command -v nodadark-tui &>/dev/null; then
    ok "nodadark-tui installé → $(which nodadark-tui)"
fi

# ═══════════════════════════════════════════════════════════
# ÉTAPE 4 — Génération du certificat CA
# ═══════════════════════════════════════════════════════════
step "ÉTAPE 4 — Génération du certificat CA"

mkdir -p "$CA_DIR"

# Lancer le moteur brièvement pour générer le CA
info "Génération du certificat CA..."
timeout 5 nodadark --port 18080 --api-port 19090 \
    --socket /tmp/nodadark_install.sock \
    --log-level warn 2>/dev/null &
NODADARK_PID=$!
sleep 3
kill $NODADARK_PID 2>/dev/null || true
wait $NODADARK_PID 2>/dev/null || true

if [ -f "$CA_DIR/nodadark-ca.crt" ]; then
    ok "Certificat CA généré : $CA_DIR/nodadark-ca.crt"
    # Copier dans le stockage Android pour installation facile
    if [ -d "/sdcard/Download" ]; then
        cp "$CA_DIR/nodadark-ca.crt" "/sdcard/Download/nodadark-ca.crt"
        ok "CA copié dans /sdcard/Download/nodadark-ca.crt (pour installation Android)"
    fi
else
    warn "CA pas encore généré — il sera créé au premier lancement"
fi

# ═══════════════════════════════════════════════════════════
# ÉTAPE 5 — Créer les fichiers de configuration par défaut
# ═══════════════════════════════════════════════════════════
step "ÉTAPE 5 — Configuration par défaut"

mkdir -p "$RULES_DIR"

# Créer rules.toml si inexistant
if [ ! -f "$RULES_DIR/rules.toml" ]; then
    cat > "$RULES_DIR/rules.toml" << 'TOML'
# NodaDark — Règles persistantes
# Fichier chargé au démarrage du moteur
# Modifier enabled = true pour activer une règle

# Exemple 1 — Supprimer X-Frame-Options
[[rules]]
name    = "Supprimer X-Frame-Options"
enabled = false
action  = { type = "remove_header", name = "X-Frame-Options" }

# Exemple 2 — Modifier User-Agent
[[rules]]
name    = "Fake User-Agent"
enabled = false
action  = { type = "modify_header", name = "User-Agent", value = "NodaDark Audit v0.1" }

# Exemple 3 — Bloquer tracker
[[rules]]
name    = "Bloquer tracker"
enabled = false
domain  = "*.analytics.com"
action  = { type = "drop" }
TOML
    ok "Fichier rules.toml créé : $RULES_DIR/rules.toml"
else
    ok "rules.toml existant conservé"
fi

# ═══════════════════════════════════════════════════════════
# ÉTAPE 6 — Alias utiles dans .bashrc
# ═══════════════════════════════════════════════════════════
step "ÉTAPE 6 — Ajout des alias"

BASHRC="$HOME/.bashrc"

# Vérifier si les alias existent déjà
if grep -q "nodadark-install" "$BASHRC" 2>/dev/null; then
    ok "Alias déjà présents dans .bashrc"
else
    cat >> "$BASHRC" << 'ALIASES'

# ── NodaDark aliases ────────────────────────────────────────
# Réinstaller les binaires après recompilation
alias nodadark-install='cp ~/nodadark/target/release/nodadark $PREFIX/bin/ && cp ~/nodadark/target/release/nodadark-tui $PREFIX/bin/ && echo "✅ NodaDark réinstallé"'

# Lancer le moteur en arrière-plan
alias nd='nodadark --port 8080 &'

# Lancer le TUI
alias nd-tui='nodadark-tui --port 9090'

# Vérifier si le moteur tourne
alias nd-status='ps aux | grep "[n]odadark --port" && echo "🟢 Moteur actif" || echo "🔴 Moteur arrêté"'

# Arrêter le moteur
alias nd-stop='pkill -f "nodadark --port" && echo "■ Moteur arrêté"'
# ────────────────────────────────────────────────────────────
ALIASES
    ok "Alias ajoutés dans .bashrc"
    source "$BASHRC" 2>/dev/null || true
fi

# ═══════════════════════════════════════════════════════════
# RÉSUMÉ FINAL
# ═══════════════════════════════════════════════════════════
echo ""
echo -e "${BOLD}${GREEN}═══════════════════════════════════════════════${RESET}"
echo -e "${BOLD}${GREEN}  ✅ NodaDark installé avec succès !${RESET}"
echo -e "${BOLD}${GREEN}═══════════════════════════════════════════════${RESET}"
echo ""
echo -e "${CYAN}COMMANDES DISPONIBLES :${RESET}"
echo -e "  ${BOLD}nodadark --help${RESET}          → Aide complète"
echo -e "  ${BOLD}nodadark --version${RESET}       → Version"
echo -e "  ${BOLD}nodadark --port 8080 &${RESET}   → Lancer le moteur"
echo -e "  ${BOLD}nodadark-tui --port 9090${RESET} → Lancer le TUI"
echo ""
echo -e "${CYAN}ALIAS RAPIDES :${RESET}"
echo -e "  ${BOLD}nd${RESET}              → Lancer moteur (port 8080)"
echo -e "  ${BOLD}nd-tui${RESET}          → Lancer TUI"
echo -e "  ${BOLD}nd-status${RESET}       → Vérifier si moteur actif"
echo -e "  ${BOLD}nd-stop${RESET}         → Arrêter le moteur"
echo -e "  ${BOLD}nodadark-install${RESET} → Réinstaller après recompilation"
echo ""
echo -e "${CYAN}CERTIFICAT CA :${RESET}"
echo -e "  ${BOLD}$CA_DIR/nodadark-ca.crt${RESET}"
if [ -f "/sdcard/Download/nodadark-ca.crt" ]; then
    echo -e "  Copié dans : ${BOLD}/sdcard/Download/nodadark-ca.crt${RESET}"
    echo -e "  → Paramètres Android → Sécurité → Installer depuis la mémoire"
fi
echo ""
echo -e "${YELLOW}⚠  Recharge le terminal pour activer les alias :${RESET}"
echo -e "  ${BOLD}source ~/.bashrc${RESET}"
echo ""
echo -e "${CYAN}DÉMARRAGE RAPIDE :${RESET}"
echo -e "  ${BOLD}source ~/.bashrc && nd${RESET}   → Lancer le moteur"
echo -e "  ${BOLD}nd-tui${RESET}                   → Nouvelle session, lancer TUI"
echo ""
echo -e "  Docs : https://github.com/roscpy/nodadark"
echo -e "${BOLD}${CYAN}  NodaDark — Intercept · Analyze · Modify · Replay${RESET}"
echo ""
