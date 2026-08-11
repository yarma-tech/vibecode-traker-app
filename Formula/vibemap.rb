# Formule Homebrew du daemon Vibe Map.
#
# Elle installe le binaire deja compile publie par le workflow Release (tag
# `v*`), plutot que de compiler chez l'utilisateur : l'ADR 0001 fait du binaire
# publie l'artefact distribue. La somme de controle sha256 verrouille l'archive.
#
# Ou vit cette formule
# --------------------
# Ce fichier est la source ; la copie qui sert reellement vit dans le tap
# `yarma-tech/homebrew-vibemap`, ou `brew tap` la trouve. C'est la voie
# « une commande » annoncee au README et a l'ecran d'appairage :
#   brew install yarma-tech/vibemap/vibemap
# ce qui equivaut a `brew tap yarma-tech/vibemap` puis `brew install vibemap`.
#
# Homebrew refuse desormais une formule hors tap (`brew install --formula
# ./Formula/vibemap.rb` sort « Homebrew requires formulae to be in a tap ») :
# passer par le tap est le seul chemin. Toute modification ici doit etre
# recopiee dans le tap.
#
# A chaque release : mettre a jour `version`, les `url` et les `sha256` avec les
# valeurs des archives et de leurs fichiers `.sha256` publies. Les sommes
# ci-dessous sont celles de la release v0.1.0.
class Vibemap < Formula
  desc "Daemon local de Vibe Map qui observe les agents pour Supabase"
  homepage "https://github.com/yarma-tech/vibecode-traker-app"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/yarma-tech/vibecode-traker-app/releases/download/v#{version}/vibemap-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "6cf3a87e04d2cf156736a9a3581d14a0b9480649661316aa756b8ffada147a5b"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/yarma-tech/vibecode-traker-app/releases/download/v#{version}/vibemap-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "310a581ce50520d147046426fd1a0b171304c6b1d9030ee4651452b8b0135a73"
    end
  end

  def install
    bin.install "vibemap"
  end

  test do
    assert_match "vibemap #{version}", shell_output("#{bin}/vibemap --version")
  end
end
