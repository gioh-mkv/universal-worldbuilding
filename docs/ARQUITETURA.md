# Arquitetura

## Visão geral

```
┌─────────────────────────── Janela (WebView) ───────────────────────────┐
│ React + TypeScript + Tailwind + TipTap                                  │
│ features/*  ──►  shared/api.ts (cliente tipado dos comandos)            │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ comandos Tauri (JSON, camelCase)
┌───────────────────────────────▼─────────────────────────────────────────┐
│ Rust                                                                    │
│ commands/  (camada fina)  ──►  domain/*  (regras)  ──►  db/  (SQLite)   │
│ state.rs: projeto aberto = Mutex<Option<OpenProject>>                   │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
            ~/Documentos/UWB/<projeto>/project.db  (SQLite + FTS5)
```

O frontend nunca acessa o disco diretamente: tudo passa por comandos Rust.

## Estrutura de pastas

```
src/                         Frontend
├─ app/                      AppContext (estado global), Sidebar
├─ features/
│  ├─ projects/              escolher, criar, duplicar e excluir projetos
│  ├─ universes/             criar, editar e excluir universos
│  ├─ entities/              lista, formulário, editor, menções, relações, backlinks
│  ├─ entity-types/          gerenciar tipos e propriedades
│  ├─ relation-types/        gerenciar tipos de relação
│  ├─ search/                pesquisa global e filtros
│  ├─ backup/                backups, verificação, restauração
│  └─ transfer/              exportar e importar
└─ shared/                   api.ts, ui.tsx, text.ts, theme.ts, dataTypes.ts

src-tauri/
├─ migrations/               001_init.sql, 002_relations_origin.sql…
└─ src/
   ├─ lib.rs                 registra plugins e comandos
   ├─ error.rs               AppError (serializado como texto para o frontend)
   ├─ state.rs               projeto aberto e troca segura do banco (restauração)
   ├─ db/                    abertura, migrações, UUID v7, SCHEMA_VERSION
   ├─ commands/              um comando por operação (sem regra de negócio)
   └─ domain/
      ├─ projects.rs         pasta do projeto, project.json, duplicar, excluir
      ├─ universes.rs
      ├─ entity_types.rs     tipos e propriedades (edição com poda de dados)
      ├─ entities.rs         CRUD, validação, índices derivados
      ├─ relations.rs        tipos de relação e relações
      ├─ links.rs            menções de entrada e saída
      ├─ richtext.rs         documentos TipTap: menções, Markdown
      ├─ search.rs           FTS5 e filtros
      ├─ backup.rs           backup, verificação, restauração, automático
      └─ transfer.rs         exportação e importação
```

## Decisões de projeto

**Um banco SQLite por projeto, vários universos dentro dele.** Simplifica backup e duplicação (uma pasta) e permite que universos do mesmo projeto convivam.

**Modelo genérico em vez de uma tabela por conceito.** Os tipos são dados: `entity_types` + `property_defs` definem os campos, e os valores ficam em uma coluna JSON (`entities.properties`). As consultas usam `json_extract`. Assim o usuário cria conceitos que o programador não previu.

**Dados derivados na mesma transação.** Menções, relações vindas de propriedades, Markdown e índice de pesquisa são recalculados dentro da transação que grava a entidade, então nunca ficam fora de sincronia:

| Derivado | Vem de |
|---|---|
| `body_md` | `body_json` (conversor em `richtext.rs`) |
| `mentions` | nós `mention` do `body_json` |
| `relations` com `property_key` | propriedades `entity_ref` |
| `search_index` | nome, resumo, propriedades e `body_md` |

**Documento em JSON (TipTap) como fonte da verdade**, com Markdown gerado no backend. Isso evita depender de um conversor do editor e serve à pesquisa e à exportação.

**Pesquisa com FTS5**, tokenizador `unicode61 remove_diacritics 2`, consulta por prefixo e ordenação `bm25` com pesos (nome 10, resumo 5, propriedades 2, corpo 1). O texto digitado é sanitizado antes de virar consulta.

**IDs em UUID v7.** Únicos e ordenáveis por data, o que facilita importação e uma eventual sincronização.

**Migrações apenas acrescentam.** `PRAGMA user_version` guarda a versão aplicada. Uma migração já publicada nunca é editada (veja [MODELO-DE-DADOS.md](MODELO-DE-DADOS.md)).

## Estado e concorrência

O projeto aberto fica em `AppState` (`Mutex<Option<OpenProject>>`). Os comandos usam `state.with(...)`. Operações pesadas (backup, verificação, exportação, importação, duplicação) são `async` para não travar a interface. A restauração usa `state.swap(...)`, que fecha a conexão, troca o arquivo e **sempre reabre** o banco, mesmo se algo falhar.

## Segurança e robustez

- Os ids de backup viram nomes de pasta, então só aceitam `[A-Za-z0-9_-]` (sem `..` ou `/`).
- Excluir ou duplicar projeto só aceita pastas **diretamente dentro** da pasta de projetos e que tenham `project.json`.
- Importações são validadas por completo antes de gravar e acontecem em **uma transação**.
- Chaves de propriedade viram caminhos JSON, então são restritas a `[A-Za-z0-9_]`.
- Nenhum dado sai do computador e o app não faz requisições de rede.
- Restaurar só prossegue se o backup passar na verificação e não for de um esquema mais novo.

## Como adicionar uma funcionalidade

1. Precisa de coluna ou tabela nova? Crie `migrations/00N_nome.sql` e acrescente-a a `MIGRATIONS` em `db/mod.rs`.
2. Escreva a regra em `domain/<modulo>.rs`, com **testes** no mesmo arquivo.
3. Exponha em `commands/mod.rs` e registre em `lib.rs`.
4. Acrescente tipos e chamada em `src/shared/api.ts`.
5. Crie a interface em `src/features/<modulo>/`.
6. Atualize o `CHANGELOG.md` e a documentação afetada.