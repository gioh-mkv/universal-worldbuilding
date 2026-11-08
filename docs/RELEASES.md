# Como publicar uma versão

## Regras de versionamento (SemVer: `MAJOR.MINOR.PATCH`)

| Mudança | Sobe |
|---|---|
| Correção de erro, sem mudar dados ou comportamento | `PATCH` (0.1.0 → 0.1.1) |
| Funcionalidade nova compatível | `MINOR` (0.1.0 → 0.2.0) |
| Mudança incompatível (formato de dados, exportação) | `MAJOR` (e, enquanto for 0.x, o `MINOR`) |

- Enquanto a versão for **0.x**, o app é considerado experimental. A **1.0.0** é o compromisso de não quebrar os dados dos usuários.
- Pré-lançamentos usam sufixo: `v0.2.0-beta.1`, `v0.2.0-rc.1`. A automação os marca como "pre-release".
- Toda tag tem o formato **`vX.Y.Z`** e a versão aparece em **três lugares** que precisam ser iguais: `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json`. O `scripts/bump-version.mjs` atualiza os três.
- **Compatibilidade de dados:** migrações só se acrescentam (nunca editadas); mudanças incompatíveis no arquivo de exportação sobem `formatVersion`.

## Checklist de release

- [ ] A `main` está verde no CI.
- [ ] O `CHANGELOG.md` tem a seção da versão, com data (mova o conteúdo de **Não lançado**).
- [ ] `node scripts/bump-version.mjs X.Y.Z`
- [ ] Commit `chore(release): vX.Y.Z` e tag anotada `vX.Y.Z`; push da `main` e da tag.
- [ ] O workflow **Release** termina e cria um **rascunho** com RPM, DEB e AppImage.
- [ ] Baixe o RPM do rascunho, instale em um Fedora limpo e teste o fluxo básico (criar projeto, entidade, backup, exportar).
- [ ] Se o projeto tem migrações novas, abra um projeto criado na versão anterior e confira que ele migra.
- [ ] Revise as notas e clique em **Publish release**.
- [ ] Depois: crie a seção **Não lançado** vazia no CHANGELOG e feche o milestone.

## Se algo der errado

Rascunho com problema: apague o rascunho, apague a tag, corrija e refaça.

```bash
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
```

Nunca reutilize uma versão já **publicada**: corrija e lance `X.Y.(Z+1)`.