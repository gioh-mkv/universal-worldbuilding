# Changelog

Todas as mudanças relevantes deste projeto são registradas aqui.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o projeto adota o [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [Não lançado]

## [0.1.0] - 2026-11-07

Primeira versão pública.

### Adicionado

- Projetos e universos: criar, editar, duplicar e excluir, com um banco SQLite por projeto.
- Tipos de entidade personalizados com 11 tipos de propriedade; edição de tipos (renomear, reordenar, obrigatoriedade, opções, remoção).
- Entidades com formulário dinâmico, validação e histórico de versões gravado a cada edição.
- Editor de documentos (TipTap) com menções `[[Entidade]]`, backlinks e atualização automática de nomes.
- Tipos de relação com nome inverso e direcionalidade; relações com datas; relações geradas automaticamente por propriedades de referência.
- Pesquisa global (FTS5) por prefixo e sem diferenciar acentos, com filtros combináveis.
- Backup manual e automático, restauração reversível, verificação de integridade (SHA-256) e retenção configurável.
- Exportação em JSON e Markdown (Obsidian) e importação de JSON.
- Tema claro, escuro e automático; barra lateral recolhível.
- Instalador RPM para Fedora.

[Não lançado]: https://github.com/gioh-mkv/universal-worldbuilding/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/gioh-mkv/universal-worldbuilding/releases/tag/v0.1.0