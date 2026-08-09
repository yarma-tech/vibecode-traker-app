# Formule Homebrew du daemon Vibe Map.
#
# Elle installe le binaire deja compile publie par le workflow Release (tag
# `v*`), plutot que de compiler chez l'utilisateur : l'ADR 0001 fait du binaire
# publie l'artefact distribue. La somme de controle sha256 verrouille l'archive.
#
# Ou vit cette formule
# --------------------
# Deux facons de s'en servir :
#
#   1. Depuis ce depot (sans tap dedie) :
#        brew install --formula ./Formula/vibemap.rb
#
#   2. Via un tap, la voie « une commande » annoncee au README et a l'ecran
#      d'appairage. La formule doit alors vivre dans un depot nomme
#      `homebrew-vibemap` sous l'organisation, ou `brew tap` la trouve :
#        brew install yarma-tech/vibemap/vibemap
#      ce qui equivaut a `brew tap yarma-tech/vibemap` puis
#      `brew install vibemap`.
#
# A chaque release : mettre a jour `version`, les `url` et les `sha256` avec les
# valeurs des archives et de leurs fichiers `.sha256` publies. Les sommes
# ci-dessous sont des gabarits (des zeros) tant qu'aucune release reelle n'a ete
# taguee ; la premiere release fournira les vraies.
class Vibemap < Formula
  desc "Daemon local de Vibe Map qui observe les agents pour Supabase"
  homepage "https://github.com/yarma-tech/vibecode-traker-app"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/yarma-tech/vibecode-traker-app/releases/download/v#{version}/vibemap-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/yarma-tech/vibecode-traker-app/releases/download/v#{version}/vibemap-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "vibemap"
  end

  test do
    assert_match "vibemap #{version}", shell_output("#{bin}/vibemap --version")
  end
end
