# Como contribuir

Obrigada por querer ajudar! Todo tipo de contribuição vale: relatar erros, sugerir ideias, melhorar a documentação e enviar código.

## Relatando um erro

Abra uma [issue](https://github.com/gioh-mkv/universal-worldbuilding/issues/new) com:

- a versão do UWB (veja na página de Releases ou no instalador) e sua distribuição Linux;
- o que você fez, o que esperava e o que aconteceu (mensagens de erro ajudam muito);
- se possível, os passos para reproduzir.

**Nunca anexe seus projetos ou backups** se eles tiverem conteúdo que você não quer tornar público.

## Sugerindo uma funcionalidade

Abra uma issue descrevendo o problema que você quer resolver. Veja antes o [roadmap](docs/ROADMAP.md): a ideia pode já estar planejada.

## Enviando código

1. Faça um fork e crie uma branch: `git switch -c feat/minha-ideia`.
2. Prepare o ambiente: [docs/DESENVOLVIMENTO.md](docs/DESENVOLVIMENTO.md).
3. Faça mudanças pequenas e focadas.
4. Antes de abrir o PR, rode:

   ```bash
   npm run build
   cargo test --manifest-path src-tauri/Cargo.toml
   ```

5. Escreva os commits no estilo `feat:`, `fix:`, `docs:`… e atualize o `CHANGELOG.md` (seção **Não lançado**).
6. Abra o Pull Request explicando **o que** mudou e **por quê**.

## Regras importantes

- **Não edite migrações já publicadas** (`src-tauri/migrations/`). Crie uma nova.
- Regras de negócio novas em `src-tauri/src/domain/` precisam de testes.
- Mantenha os dados locais: o app não deve fazer requisições de rede nem enviar conteúdo do usuário a serviços externos.