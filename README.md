# Universal Worldbuilding (UWB)

> Construa, documente e conecte universos ficcionais: no seu computador, sem internet e sem categorias impostas.

[![CI](https://github.com/gioh-mkv/universal-worldbuilding/actions/workflows/ci.yml/badge.svg)](https://github.com/gioh-mkv/universal-worldbuilding/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/gioh-mkv/universal-worldbuilding?include_prereleases)](https://github.com/gioh-mkv/universal-worldbuilding/releases)
[![Licença: MIT](https://img.shields.io/badge/licen%C3%A7a-MIT-blue.svg)](LICENSE)

<!-- Adicione capturas de tela em docs/img/ e referencie aqui:
![Tela principal](docs/img/tela-principal.png) -->

## O que é

O UWB é um aplicativo de desktop para escritores, roteiristas, quadrinistas, ilustradores e pesquisadores que criam mundos ficcionais de qualquer gênero: fantasia, ficção científica, cyberpunk, história alternativa, realidades híbridas.

Em vez de fichas fixas de "personagem" ou "cidade", **você define os seus próprios tipos de entidade e as propriedades de cada um**. Tudo o que você cadastra pode se relacionar com todo o resto, ser mencionado em documentos e ser encontrado pela pesquisa.

## Princípios

- **Local-first:** seus dados ficam no seu computador e o app funciona sem internet.
- **Flexível:** nenhuma categoria temática é obrigatória.
- **Interconectado:** entidades, documentos e relações compartilham os mesmos dados.
- **Portátil:** você exporta tudo (JSON e Markdown) e faz backups verificáveis.

## Funcionalidades (versão 0.1)

- **Projetos e universos:** um projeto pode conter vários universos independentes.
- **Tipos de entidade personalizados**, com propriedades de 11 tipos: texto, número, data, data e hora, sim/não, lista, múltipla seleção, referência a outra entidade, coordenadas, imagem e arquivo (caminho).
- **Editor de documentos** em cada entidade, com títulos, listas, citações e **menções `[[Entidade]]`** com painel de backlinks.
- **Relações tipadas** com nome inverso (por exemplo "pertence a" / "possui"), direcionalidade e datas de início e fim.
- **Pesquisa global** (`Ctrl+K`) por prefixo, sem diferenciar acentos, com filtros combináveis por tipo, relação, menção e propriedade.
- **Edição e exclusão seguras**, que preservam a consistência (menções viram texto, referências são limpas).
- **Backup e restauração** com verificação de integridade (SHA-256), backups automáticos e retenção configurável.
- **Exportação** em JSON (reimportável) e Markdown (compatível com o Obsidian), e **importação** de JSON.
- Tema claro, escuro ou automático.

## Estado do projeto

O UWB está em **desenvolvimento inicial (0.x)**. A primeira entrega cobre o núcleo de dados. Grafo de relações, cronologias, calendários próprios e mapas estão no [roadmap](docs/ROADMAP.md). Enquanto a versão for 0.x, faça backups com frequência e leia o [CHANGELOG](CHANGELOG.md) antes de atualizar.

Plataforma prioritária: **Linux** (desenvolvido e testado no Fedora).

## Instalação

Baixe o instalador na página de [**Releases**](https://github.com/gioh-mkv/universal-worldbuilding/releases).

**Fedora (RPM):**

```bash
cd pasta-onde-baixou
sudo dnf install ./Universal*.rpm
```

**Debian, Ubuntu e Linux Mint (DEB):**

```bash
sudo apt install ./Universal*.deb
```

**Qualquer distribuição (AppImage):**

```bash
chmod +x Universal*.AppImage
./Universal*.AppImage
```

Para desinstalar no Fedora: `rpm -qa | grep -i universal` mostra o nome do pacote e `sudo dnf remove NOME` o remove. Seus projetos **não** são apagados.

Para compilar a partir do código-fonte, veja [docs/DESENVOLVIMENTO.md](docs/DESENVOLVIMENTO.md).

## Primeiros passos

1. Abra o app e crie um **projeto** (por exemplo, "Crônicas de Aurora").
2. Crie um **universo** dentro dele.
3. Em **Tipos de entidade**, defina o que existe no seu mundo: "Personagem" com `Idade` (número) e `Espécie` (lista), "Cidade" com `Fundador` (referência a Personagem), e assim por diante.
4. Em **Entidades**, cadastre registros e escreva o documento de cada um. Digite `[[` para mencionar outra entidade.
5. Conecte as entidades em **Nova relação** e encontre tudo com `Ctrl+K`.
6. Faça seu primeiro backup em **Backup**.

O passo a passo completo está no [Guia do usuário](docs/GUIA-DO-USUARIO.md).

## Onde ficam seus dados

Cada projeto é uma pasta em `~/Documentos/UWB/` (ou `~/UWB/`, se a pasta Documentos não existir):

```
~/Documentos/UWB/cronicas-de-aurora/
├─ project.json     nome do projeto e configurações de backup
├─ project.db       banco de dados SQLite
├─ assets/          arquivos anexados (reservado)
└─ backups/         backups do projeto
```

Para levar um projeto a outro computador, copie a pasta inteira. Para fazer uma cópia de segurança fora do app, copie também `backups/` para outro disco.

## Documentação

| Documento | Para quem |
|---|---|
| [Guia do usuário](docs/GUIA-DO-USUARIO.md) | Quem usa o app |
| [Arquitetura](docs/ARQUITETURA.md) | Quem quer entender como o app é construído |
| [Modelo de dados](docs/MODELO-DE-DADOS.md) | Quem mexe no banco ou nos formatos de arquivo |
| [Desenvolvimento](docs/DESENVOLVIMENTO.md) | Quem vai compilar e contribuir |
| [Releases](docs/RELEASES.md) | Quem publica versões |
| [Roadmap](docs/ROADMAP.md) | O que já existe e o que vem a seguir |

## Tecnologias

[Tauri 2](https://tauri.app) · Rust · SQLite (FTS5) · React · TypeScript · Tailwind CSS · TipTap

## Contribuindo

Contribuições são bem-vindas: relatos de erro, sugestões e código. Leia o [CONTRIBUTING.md](CONTRIBUTING.md).

## Licença

[MIT](LICENSE).