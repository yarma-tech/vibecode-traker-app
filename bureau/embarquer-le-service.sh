#!/bin/sh
# Prepare ce que le paquet embarque pour servir l'interface (FR-069).
#
# Le paquet doit s'ouvrir sur la carte sur un poste qui n'a ni le depot, ni
# Node, ni les dependances de l'interface. Tout ce qu'il faut pour la servir
# entre donc dans l'application, et ce script est ce qui l'assemble :
#
#   service-embarque/server.js      le serveur sorti de `next build`
#   service-embarque/.next/         ce qu'il sert, ressources statiques comprises
#   service-embarque/node_modules/  les seules dependances qu'il touche
#   service-embarque/public/        les fichiers servis tels quels
#   service-embarque/node           l'executable qui le fait tourner
#
# `tauri.conf.json` recopie ce dossier dans `Contents/Resources/service`, et
# `bureau/src/service.rs` va l'y chercher.
#
# POURQUOI un script, et pas `build.rs` : ce qu'il assemble ne depend pas une
# ligne du code Rust. Le lancer depuis `build.rs` le rejouerait a chaque
# `cargo build`, et ferait dependre `cargo test` de `npm` et du reseau.
# `tauri.conf.json` l'appelle en `beforeBuildCommand` : il tourne quand on
# empaquette, et seulement la.
#
# POURQUOI un Node telecharge, et pas celui du poste : le Node d'un
# gestionnaire de paquets n'est souvent qu'une amorce qui va chercher une
# quinzaine de bibliotheques ailleurs sur la machine - celui de Homebrew, par
# exemple, pese 68 ko et pointe vers `/opt/homebrew/opt/...`. Recopie dans le
# paquet, il ne demarrerait que sur un poste qui a deja Homebrew et ce meme
# Node : le defaut qu'on corrige, en plus discret. Les archives officielles de
# nodejs.org, elles, ne dependent que du systeme. La version est figee ici et
# sa somme de controle avec, pour que le reseau n'ait pas voix au chapitre.
set -eu

VERSION_NODE=22.23.2
SOMME_NODE=5eff7a9011895aae3f29d06f167b84a62b028a591370c7cafb59103559fd26e1
ARCHIVE_NODE="node-v${VERSION_NODE}-darwin-arm64"

ici=$(cd "$(dirname "$0")" && pwd)
depot=$(cd "$ici/.." && pwd)
web="$depot/web"
stage="$ici/service-embarque"
cache="$ici/.node/$VERSION_NODE"

# La publication ne vise que les Mac Apple Silicon (FR-079), et le Node
# embarque est celui de cette architecture : sur une autre machine, le paquet
# produit serait muet a l'ouverture plutot qu'ici.
if [ "$(uname -s)" != "Darwin" ] || [ "$(uname -m)" != "arm64" ]; then
  echo "erreur : le paquet ne se construit que sur un Mac Apple Silicon (FR-079)." >&2
  exit 1
fi

if [ ! -d "$web/node_modules" ]; then
  echo "erreur : les dependances de l'interface manquent." >&2
  echo "        lance \`npm install\` dans web/, puis reessaie." >&2
  exit 1
fi

# --- L'interface -----------------------------------------------------------

# La sortie autonome rassemble le serveur et les seules dependances qu'il
# touche. `web/next.config.ts` ne la reclame que si cette variable est posee -
# le site heberge et la voie de developpement continuent de servir `next start`
# comme avant. Elle ne recopie deliberement ni `.next/static` ni `public/`, que
# son hebergeur habituel sert par ailleurs : ici, personne d'autre ne les
# servira.
echo "> construction de l'interface"
(cd "$web" && VIBEMAP_SERVICE_EMBARQUE=1 npm run build)

autonome="$web/.next/standalone"
if [ ! -f "$autonome/server.js" ]; then
  echo "erreur : $autonome/server.js est absent apres la construction." >&2
  echo "        verifie que web/next.config.ts sort bien en \"standalone\" quand" >&2
  echo "        VIBEMAP_SERVICE_EMBARQUE est pose." >&2
  exit 1
fi

# Garde-fou avant l'effacement : on ne vide que le dossier qu'on a fabrique.
case "$stage" in
*/bureau/service-embarque) ;;
*)
  echo "erreur : $stage n'est pas le dossier attendu, rien n'est efface." >&2
  exit 1
  ;;
esac

echo "> assemblage dans $stage"
rm -rf "$stage"
mkdir -p "$stage"
cp -R "$autonome/." "$stage/"
cp -R "$web/.next/static" "$stage/.next/static"
cp -R "$web/public" "$stage/public"

# --- L'executable qui la fait tourner --------------------------------------

if [ ! -x "$cache/node" ]; then
  echo "> telechargement de Node $VERSION_NODE"
  mkdir -p "$cache"
  archive="$cache/$ARCHIVE_NODE.tar.xz"
  curl -fsSL --retry 3 -o "$archive" \
    "https://nodejs.org/dist/v${VERSION_NODE}/${ARCHIVE_NODE}.tar.xz"

  somme=$(shasum -a 256 "$archive" | cut -d' ' -f1)
  if [ "$somme" != "$SOMME_NODE" ]; then
    rm -f "$archive"
    echo "erreur : l'archive Node telechargee n'est pas celle attendue." >&2
    echo "        attendue $SOMME_NODE" >&2
    echo "        obtenue  $somme" >&2
    exit 1
  fi

  # Seul l'executable est extrait : le reste de l'archive (npm, en-tetes,
  # documentation) ne sert a rien pour faire tourner un serveur deja construit.
  tar -xJf "$archive" -C "$cache" --strip-components 2 "$ARCHIVE_NODE/bin/node"
  rm -f "$archive"
fi

cp "$cache/node" "$stage/node"
chmod +x "$stage/node"

# Ce que le paquet ne doit RIEN devoir au poste : si l'executable embarque
# reclame une bibliotheque hors du systeme, il ne demarrera que sur une machine
# qui l'a deja. Mieux vaut echouer ici que livrer un paquet qui ne s'ouvre que
# chez celui qui l'a construit.
emprunts=$(otool -L "$stage/node" | tail -n +2 | awk '{print $1}' |
  grep -v -E '^(/usr/lib/|/System/)' || true)
if [ -n "$emprunts" ]; then
  echo "erreur : le Node embarque depend de bibliotheques hors du systeme :" >&2
  echo "$emprunts" >&2
  exit 1
fi

version_vue=$("$stage/node" -p 'process.versions.node + " " + process.arch')
echo "> service pret : Node $version_vue, $(du -sh "$stage" | cut -f1)"
