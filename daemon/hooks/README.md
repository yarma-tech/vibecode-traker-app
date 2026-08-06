# Le hook `PostToolUse`, facultatif

Sans lui, tout fonctionne : le daemon lit `~/.claude/projects/**/*.jsonl` toutes
les deux secondes et y trouve les mêmes appels d'outils. Le hook ne fait qu'une
chose, poster l'appel au moment où il se produit plutôt qu'au tour suivant. Il
fait gagner une seconde environ, pas une fonctionnalité.

Les deux chemins portent le même `tool_use_id`. La contrainte
`unique (session_id, tool_use_id)` absorbe le doublon : le premier arrivé gagne,
l'autre est ignoré sans erreur.

## Poser le hook

Dans `~/.claude/settings.json` :

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Read|Grep|Glob|Edit|Write|NotebookEdit",
        "hooks": [{ "type": "command", "command": "vibemap hook" }]
      }
    ]
  }
}
```

`vibemap` doit être dans le `PATH` (voir l'issue #14). À défaut, donner le
chemin complet du binaire.

Le `matcher` n'est qu'une économie : `vibemap hook` se tait de lui-même pour
tout outil hors correspondance, `Bash` en tête.

## Ce qu'il fait, ce qu'il ne fait pas

Il lit la charge utile sur son entrée standard, en tire la session, l'outil et
le chemin, retrouve le repo par l'empreinte de sa racine, et poste. Aucun
contenu de fichier, aucun prompt, aucun résultat d'outil ne le traverse.

Il sort toujours en succès et n'écrit jamais sur la sortie standard : un hook ne
doit pas gêner l'agent qui l'appelle. Ses ennuis vont sur la sortie d'erreur,
visible avec `claude --debug`.

Si le repo n'a pas encore été cartographié, le hook ne dit rien : la prochaine
lecture des journaux rattrapera l'événement une fois le plan envoyé.

## Vérifier

```sh
echo '{"session_id":"essai","cwd":"'"$PWD"'","tool_name":"Edit",
       "tool_input":{"file_path":"'"$PWD"'/README.md"},
       "tool_use_id":"toolu_essai"}' | vibemap hook
```

La parcelle du dossier doit passer en ambre sur l'application web.
