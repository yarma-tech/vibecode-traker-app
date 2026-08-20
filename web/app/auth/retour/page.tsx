/**
 * La dernière page du navigateur du système (FR-072, issue #63).
 *
 * L'autorisation est revenue ici, et le code a été repris pour la fenêtre de
 * l'application, seule à pouvoir le finir. Il ne reste rien à faire dans le
 * navigateur - et surtout rien à recopier : c'est la promesse de FR-072.
 *
 * Cette page ne dit pas si l'autorisation a réussi ou échoué. Elle ne le sait
 * pas, et elle n'a pas à le savoir : ce qui a été rapporté appartient à la
 * fenêtre, qui l'annonce là où l'utilisateur va regarder.
 */
export default function Retour() {
  return (
    <main className="accueil">
      <h1>C&apos;est bon</h1>
      <p className="pitch">
        Tu peux fermer cet onglet : la suite se passe dans la fenêtre de Vibe Map, qui vient de
        reprendre la main.
      </p>
    </main>
  );
}
