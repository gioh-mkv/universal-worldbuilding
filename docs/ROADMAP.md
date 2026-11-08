# Roadmap

Baseado no documento de requisitos do UWB (versão 1.0).

## Critérios de aceitação da primeira versão

| # | Critério | Estado |
|---|---|---|
| 1 | Múltiplos projetos e universos | ✅ |
| 2 | Categorias de entidade próprias por universo | ✅ |
| 3 | Propriedades personalizadas | ✅ |
| 4 | Entidades se relacionam livremente | ✅ |
| 5 | Referências internas entre registros | ✅ (menções `[[ ]]`) |
| 6 | Grafos de relacionamentos | ⬜ Fase 2 |
| 7 | Criar e consultar cronologias | ⬜ Fase 2 |
| 8 | Mapas com camadas, marcadores e referências | ⬜ Fase 2 |
| 9 | Conteúdo disponível offline | ✅ |
| 10 | Exportar e restaurar projetos | ✅ (backup/restauração e JSON) |
| 11 | Funciona nas distribuições Linux previstas | 🟡 testado no Fedora; Ubuntu e Mint ainda não |
| 12 | Nenhuma categoria temática obrigatória | ✅ |

## Fases

**Fase 1: Fundação**
- ✅ Projetos, universos, banco de dados, navegação
- ✅ Editor de documentos, entidades personalizadas, relacionamentos, pesquisa global
- ⬜ Dashboard personalizável

**Fase 2: Visualização**
- ⬜ Grafo de relações
- ⬜ Diagramas hierárquicos e livres
- ⬜ Cronologias e calendários personalizados
- ⬜ Mapas e camadas

**Fase 3: Organização avançada**
- ⬜ Organizações e estruturas hierárquicas
- ⬜ Planejamento narrativo (obras, capítulos, cenas)
- ⬜ Biblioteca de arquivos e etiquetas
- 🟡 Histórico de alterações (os snapshots já são gravados; falta a interface)
- ⬜ Verificação de consistência narrativa

**Fase 4: Portabilidade e aprimoramento**
- ✅ Exportação JSON e Markdown, importação JSON
- ✅ Backups manuais e automáticos, restauração, verificação de integridade
- ⬜ Exportação em CSV, HTML e PDF
- ⬜ Temas personalizados, atalhos configuráveis
- ⬜ Otimização para universos muito grandes (paginação e listas virtualizadas)

**Fase 5: Recursos futuros**
- ⬜ Sincronização, colaboração, plugins, API, assistente de IA opcional, publicação web

## Próximos passos sugeridos

1. Grafo de relações
2. Cronologias e calendários personalizados
3. Biblioteca de arquivos (imagens) e etiquetas
4. Interface de histórico de versões
5. Proteção contra perda de edições não salvas
6. Mapas