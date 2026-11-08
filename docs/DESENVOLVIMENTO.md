# Desenvolvimento

## Requisitos

- **Node.js 20.19+ ou 22+** e npm
- **Rust estável 1.85 ou mais novo** (o projeto usa a edição 2024), via [rustup](https://rustup.rs)
- Dependências de sistema do Tauri:

```bash
# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel
sudo dnf group install "c-development"

# Debian, Ubuntu e Linux Mint
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

## Começando

```bash
git clone https://github.com/gioh-mkv/universal-worldbuilding.git
cd universal-worldbuilding
npm install
npm run tauri dev
```

A primeira compilação demora alguns minutos (o Cargo compila o SQLite embutido). As seguintes são rápidas.

## Comandos

| Comando | O que faz |
|---|---|
| `npm run tauri dev` | Abre o app com recarga automática |
| `npx tsc --noEmit` | Verifica os tipos do frontend |
| `npm run build` | Verifica os tipos e gera o frontend (`dist/`) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Roda os testes do backend (19) |
| `npm run tauri build` | Gera o instalador (RPM, conforme `tauri.conf.json`) |
| `npm run tauri build -- --bundles appimage` | Gera outro formato |
| `node scripts/bump-version.mjs X.Y.Z` | Atualiza a versão nos três lugares |

O instalador fica em `src-tauri/target/release/bundle/`.

## Testes

Os testes do backend ficam no mesmo arquivo do código (`#[cfg(test)]`) e usam bancos em memória ou pastas temporárias. Toda regra nova em `domain/` deve ter teste. O frontend ainda não tem testes automatizados, mas `tsc` precisa passar sem erros.

## Convenções

- **Commits** no estilo [Conventional Commits](https://www.conventionalcommits.org/pt-br/): `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- **Branches:** `feat/nome-curto`, `fix/nome-curto`. A `main` deve estar sempre estável.
- **Idioma:** interface, documentação e mensagens de erro em português do Brasil; código e identificadores em inglês.
- **Banco:** nunca edite uma migração já publicada; crie uma nova.
- Comandos Tauri recebem e devolvem JSON em `camelCase` (`#[serde(rename_all = "camelCase")]`).

## Problemas comuns

| Sintoma | Causa e solução |
|---|---|
| `npm: comando não encontrado` | Instale o Node: `sudo dnf install nodejs npm` |
| `Failed to resolve import …` | Falta um arquivo em `src/`; confira com `find src -type f` |
| `Command … not found` na interface | O backend está desatualizado; reinicie `npm run tauri dev` |
| Janela em branco no Fedora | Teste `WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri dev` |
| `cargo test` falha por `frontendDist` | Rode `npm run build` antes |
| Build de produção para nos tipos | Rode `npx tsc --noEmit` e corrija os erros |