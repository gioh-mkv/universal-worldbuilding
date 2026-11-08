# Guia do usuário

## Conceitos

| Conceito | O que é |
|---|---|
| **Projeto** | Uma pasta no seu computador com um banco de dados. Reúne um ou mais universos. |
| **Universo** | Um mundo ficcional. Tem seus próprios tipos, entidades e relações. |
| **Tipo de entidade** | Um molde que você define ("Personagem", "Planeta", "Corporação"), com suas propriedades. |
| **Propriedade** | Um campo de um tipo (Idade, Espécie, Fundador…). |
| **Entidade** | Um registro de um tipo ("Elena Voss", "Aurora"). Tem propriedades e um documento. |
| **Relação** | Uma ligação nomeada entre duas entidades ("Elena → pertence a → Aurora"). |
| **Menção** | Um link `[[Nome]]` dentro de um documento que aponta para outra entidade. |

## Seu primeiro universo

1. **Projeto:** na tela inicial, digite um nome e clique em *Criar projeto*.
2. **Universo:** a barra lateral tem o seletor de universo. Clique em **+** para criar outro.
3. **Tipos de entidade** (menu *Tipos de entidade*):
   - *Personagem*: `Idade` (Número), `Espécie` (Lista de opções: Humano, Elfo, Androide), `Ocupação` (Texto, obrigatória).
   - *Cidade*: `Fundador` (Referência a entidade, somente Personagem).
4. **Entidades** (menu *Entidades*): clique em *+ Nova entidade*, escolha o tipo e preencha. Os campos mudam conforme o tipo.
5. **Documento:** abaixo das propriedades fica o editor. Use a barra de ferramentas (negrito, itálico, subtítulo, listas, citação).
6. **Menções:** digite `[[`, escolha uma entidade com as setas e `Enter`. Ela vira um link azul.
7. **Relações:** na página de uma entidade já salva, *+ Nova relação*.
8. **Pesquisa:** `Ctrl+K`.

## Tipos de propriedade

| Tipo | Observação |
|---|---|
| Texto | Qualquer texto curto. |
| Número | Inteiro ou decimal. |
| Data / Data e hora | Seletor de data. As datas de um calendário próprio virão com os calendários personalizados. |
| Sim / Não | Caixa de seleção. |
| Lista de opções | Uma opção, entre as que você definir (separadas por vírgula). |
| Múltipla seleção | Várias opções. |
| Referência a entidade | Aponta para outra entidade (opcionalmente só de um tipo). **Gera uma relação automática.** |
| Coordenadas | Dois números (latitude/Y e longitude/X). |
| Imagem / Arquivo | Por enquanto guarda apenas um caminho. A biblioteca de arquivos virá depois. |

Em um tipo existente você pode renomear propriedades, marcar como obrigatória, reordenar, adicionar e remover. A **chave** e o **tipo de dado** de uma propriedade não mudam depois de criados: para mudar, remova e recrie.

## Menções e backlinks

- Digite `[[` no documento, filtre pelo nome e escolha.
- `Ctrl+clique` em uma menção abre a entidade.
- A página de cada entidade mostra **Mencionada em** (backlinks) e **Menciona**.
- **Renomear** uma entidade atualiza o texto das menções nos outros documentos.
- **Excluir** uma entidade transforma as menções a ela em texto simples.

## Relações

Cada **tipo de relação** tem um nome, um nome inverso e uma direcionalidade:

| Direcionalidade | Exemplo | Como aparece |
|---|---|---|
| Direcional | "pertence a" / "possui" | Elena → pertence a Aurora; Aurora ← possui Elena |
| Bidirecional | "é aliado de" | Elena ↔ Aurora, dos dois lados |
| Hierárquica | "comanda" / "é comandado por" | A origem fica acima do destino |

- Dá para informar **início** e **fim** (texto livre, por exemplo "2145").
- Relações vindas de uma propriedade de referência aparecem marcadas **via propriedade** e só mudam editando a propriedade.
- Gerencie os tipos em *Tipos de relação*. Um tipo em uso não pode ser excluído.

## Pesquisa

`Ctrl+K` de qualquer tela.

- Busca **por prefixo** (`aur` acha Aurora) e **sem diferenciar acentos** (`jose` acha José).
- Procura em nomes, resumos, propriedades e no texto dos documentos, com trechos destacados.
- Os **filtros se combinam** (E): por tipo, "relacionada a" (opcionalmente por tipo de relação), "menciona" e igualdade de propriedade.
- Sem texto, os filtros funcionam como uma listagem.

## Backup e restauração

Menu *Backup*:

- **Criar backup:** gera uma cópia consistente do banco, verificada na hora.
- **Verificar:** confere o checksum (SHA-256) e a integridade do banco de um backup.
- **Restaurar:** volta o projeto ao estado do backup. Antes, o estado atual é guardado como backup "Antes de restaurar", então a restauração é reversível.
- **Automático:** ligue e defina o intervalo (1 a 720 h) e quantos manter. Só roda com o app aberto e só remove backups automáticos.

> Os backups ficam dentro da pasta do projeto. Copie a pasta `backups/` para outro disco de vez em quando: isso protege contra falha do disco.

## Exportar e importar

Menu *Exportar / Importar*:

- **JSON completo:** um arquivo com tipos, entidades, documentos e relações.
- **Markdown (Obsidian):** uma pasta com uma nota por entidade, propriedades no cabeçalho YAML e links `[[Nome]]`. É só de saída.
- **Importar JSON:** sempre cria um **novo universo** (nunca altera os existentes). Problemas leves viram avisos no resumo.

## O que acontece ao excluir

| Você exclui | Resultado |
|---|---|
| Entidade | Menções a ela viram texto simples, propriedades de referência a ela são limpas, relações e histórico dela são apagados. |
| Tipo de entidade | Só se não tiver entidades. |
| Propriedade de um tipo | O valor dela é apagado de todas as entidades do tipo (com confirmação). |
| Opção de uma lista | Os valores que a usavam ficam vazios. |
| Tipo de relação | Só se nenhuma relação o usar. |
| Universo | Apaga tudo dentro dele (exige digitar o nome). |
| Projeto | Apaga a pasta inteira, **inclusive os backups** (exige digitar o nome). |

Não há lixeira: faça um backup antes de excluir algo importante.

## Atalhos

| Atalho | Ação |
|---|---|
| `Ctrl+K` | Abrir a pesquisa |
| `[[` | Mencionar uma entidade no documento |
| `↑` `↓` `Enter` | Escolher no popup de menções |
| `Esc` | Fechar o popup / diálogos |
| `Ctrl+clique` em uma menção | Abrir a entidade mencionada |

## Limitações atuais

- Navegar para outra entidade com alterações não salvas descarta as edições (o aviso "● Alterações não salvas" indica o risco).
- Relações criadas manualmente não podem ser editadas (apague e crie de novo).
- Ainda não há grafo, cronologias, calendários personalizados, mapas, biblioteca de arquivos ou etiquetas. Veja o [roadmap](ROADMAP.md).