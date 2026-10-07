# Auditoria do CRM — ZapFast Business

Data: 07/10/2026 · Contato de teste: "Douglas Andrade (Você)" (número mascarado: 5533…6289)
Escopo: auditoria visual, funcional e técnica. **Nenhuma correção foi implementada. Nenhum commit/release foi feito.**
Nenhuma mensagem foi enviada e nenhuma conversa de outro contato foi aberta.

## Ambiente (Parte 1)

| Item | Valor |
|---|---|
| Versão do código-fonte | `zapfast` 0.19.7 (Cargo.toml) |
| Branch / commit | `business/windows-isolation` @ `777c5aa52a37df91e18cdc9401557d15859a7bd5` |
| Versão do app instalado | **Não verificada** (sem shell no Windows durante a sessão) |
| App instalado = código atual? | **Não verificado.** Indício: o arquivo da branch foi atualizado ~44 min depois da última edição de `.rs`, o que sugere commit posterior às edições |
| Alterações locais não commitadas | **Não verificado** (não foi possível executar `git status`) |
| Resolução / escala | Não lidas do sistema. Estimativa pelas capturas: monitor 16:9 (provável 1920×1080) e escala ≈ 100% (sidebar de 320 px lógicos medida ≈ 318 px) |
| Janela inicial | ≈ 1495 × 910 px (estimado) |
| Monitores | 3 (o app estava em um monitor secundário; ver CRM-010) |

Evidências: capturas de tela da sessão (não salvas em arquivo). Algumas mostram o número do contato de teste; mascarar antes de compartilhar.

### Estado original do CRM do contato de teste (registrado antes de qualquer alteração)
- Etapas existentes: **2** — "Lead" (`col_1791324838`, `#8b5cf6`, ordem 1) e "Pós-Venda" (`post`, `#6b7280`, ordem 4). As 5 etapas padrão **não** existem hoje.
- Negócio: etapa "Lead", valor R$ 0,00, sem etiquetas, sem notas, 0 follow-ups. Único negócio do funil.
- Backup JSON exportado pelo próprio app (528 bytes, texto puro): `zapfast\target\crm-audit-backup-antes.json` (pasta ignorada pelo git).

### Estado em que o app foi deixado (ATENÇÃO — restauração pendente)
A conexão com o computador caiu antes da limpeza final:
1. O negócio do contato de teste aponta para uma etapa temporária ("TESTE CRM etapa temporária") **já excluída** → o card some do funil e a sidebar mostra "Selecione". **Restaurar:** abrir a sidebar do contato e escolher "Lead" na Etapa do Funil, ou Funil → Backup/Opções → Importar Backup JSON → `crm-audit-backup-antes.json`.
2. Existe 1 follow-up temporário **concluído**: "TESTE CRM lembrete temporário" (08/10 10:16). **Restaurar:** sidebar → ícone de lixeira do lembrete (agora alcançável, pois a etiqueta longa já foi removida).
3. Valor, etiquetas e nota já voltaram ao original (R$ 0,00, sem etiquetas, sem nota) — ver CRM-001.
4. Os campos da sidebar guardam rascunhos em memória (CRM-013). Fechar e reabrir o app antes de editar para não regravar texto antigo.
5. A janela do ZapFast ficou deslocada para a esquerda na tela.

## Resumo

| ID | Prioridade | Área | Resumo | Confirmado |
|---|---|---|---|---|
| CRM-001 | Crítica | Persistência | "Adicionar ao Funil" num contato já existente apaga valor, etiquetas, notas e etapa | Visual + código |
| CRM-002 | Crítica | Notas | `&deal.notes[..60]` causa panic com acento/emoji na posição 60 | Código + reprodução isolada |
| CRM-003 | Alta | Etapas | Excluir etapa com negócios deixa negócios órfãos invisíveis, sem aviso | Visual + código |
| CRM-004 | Alta | Sidebar | Largura arrastada volta sozinha ao soltar | Visual + código |
| CRM-005 | Alta | Valor | "1.500,00" vira R$ 0,00 sem aviso; entrada inválida vira 0 silenciosamente | Visual + código |
| CRM-006 | Alta | Sidebar | Conteúdo transborda/corta; etiqueta longa esconde botões e impede remoção | Visual + código |
| CRM-007 | Alta | Métricas | "Fechamentos Ganhos" só considera etapa com id `close` | Visual (sempre 0) + código |
| CRM-008 | Média | Avatar | Ícone genérico em vez da foto na sidebar e nos cards | Visual + código |
| CRM-009 | Média | Kanban | Nome longo de etapa sobrepõe contador/botões; "Conversar" cortado | Visual + código |
| CRM-010 | Média | Backup | Diálogo nativo bloqueante, abre em outro monitor e congela o app | Visual + código |
| CRM-011 | Média | Backup | Backup JSON em texto puro (com nº de telefone e notas "criptografadas") | Visual + arquivo |
| CRM-012 | Média | Etapas | Nova etapa entra no meio do funil; ID por segundo pode colidir | Visual (posição) + isolado (ID) |
| CRM-013 | Alta | Persistência | IDs temporários globais: rascunhos vazam entre contatos | Código |
| CRM-014 | Alta | Persistência | Erros do banco ignorados (`let _ =`); salvamento otimista | Código |
| CRM-015 | Média | Notas | Nota só grava ao perder foco; "✓ Salvo" fixo | Visual ("Salvo" fixo) + código |
| CRM-016 | Média | Valor | Campo mantém texto antigo (não reflete o valor real) | Código |
| CRM-017 | Média | Etapas | "Restaurar 5 Etapas Padrão" conflita com etapas personalizadas | Código |
| CRM-018 | Média | Backup | Importação sem transação nem validação | Código |
| CRM-019 | Média | Métricas | KPIs de largura fixa (5×120) em diálogo de 680 px; percentuais incluem órfãos | Código |
| CRM-020 | Média | Kanban | Barra superior em uma linha pode transbordar em janela estreita | Código |
| CRM-021 | Baixa | Follow-ups | "+1d" soma 1 dia ao horário original; ID de lembrete pouco aleatório | Código |

---

# 1. Bugs confirmados

## CRM-001 — Re-adicionar contato apaga os dados do negócio

- Prioridade: Crítica
- Área: Persistência / Kanban
- Confirmado visualmente: Sim
- Confirmado no código: Sim
- Ambiente: Windows, janela ≈ 1135 px (captura), versão 0.19.7
- Pré-condição: contato já no funil com valor R$ 100,50, etiqueta, nota e etapa Pós-Venda
- Passos para reproduzir:
  1. Funil → "+ Novo Negócio".
  2. Filtrar "Douglas Andrade" e selecionar o contato (já está no funil).
  3. Etapa inicial "Lead", valor/etiquetas vazios → "✓ Adicionar ao Funil".
- Comportamento atual: o card volta para "Lead" com R$ 0,00, sem etiquetas e sem nota. Sem aviso nem confirmação.
- Comportamento esperado: detectar contato existente (avisar/oferecer mover) ou preservar notas/etiquetas/valor.
- Evidência: capturas antes (card com R$ 100,50 + tag + nota) e depois (card vazio, KPI "1 negócios • R$ 0,00").
- Arquivo e linhas: `src/ui/kanban.rs` L313-329 (`notes: String::new()`, `tags` do formulário) ; `src/archive/crm.rs` L161-182 (`ON CONFLICT(chat_id) DO UPDATE` sobrescreve todas as colunas); `src/app.rs` L5521-5524.
- Risco para dados: perda silenciosa de notas internas, valor e etiquetas.
- Sugestão de correção: no handler, buscar `crm_deals.get(chat_id)`; se existir, mesclar (manter notas/tags/valor quando o campo do formulário estiver vazio) ou exibir "já está no funil". Nunca construir `CrmDeal` com campos vazios para chave existente.
- Teste de regressão recomendado: criar deal com nota/tag/valor; "adicionar" de novo; verificar que os três persistem.

## CRM-002 — Panic no card do Kanban com nota contendo acento/emoji

- Prioridade: Crítica (derruba o app ao abrir o Funil)
- Área: Notas / Kanban
- Confirmado visualmente: Não (de propósito: não derrubei o app em uso)
- Confirmado no código: Sim; reproduzido em programa Rust isolado com a mesma lógica
- Ambiente: qualquer
- Pré-condição: nota com mais de 60 bytes em que o byte 60 cai no meio de um caractere (ex.: 59 caracteres ASCII + "ç", ou 58 ASCII + 🚀)
- Passos para reproduzir: 1. escrever a nota na sidebar e sair do campo; 2. abrir o Funil.
- Comportamento atual: `byte index 60 is not a char boundary` → panic. Como a nota já está gravada, o Funil quebra toda vez.
- Comportamento esperado: truncar por caracteres.
- Evidência: reprodução isolada ("PANIC" para `"x"*59+"ção…"` e `"x"*58+"🚀🚀🚀"`). A nota de exemplo do enunciado (67 bytes) **não** dispara o panic (byte 60 é fronteira válida).
- Arquivo e linhas: `src/ui/kanban.rs` L672-680 (L674-675).
- Risco para dados: não corrompe, mas bloqueia o acesso ao funil e pode perder estado não salvo do app.
- Sugestão: `deal.notes.chars().take(60).collect::<String>()` (+ "…" se `chars().count() > 60`); usar `char_indices` se precisar de bytes.
- Teste de regressão: nota com 59 ASCII + "ç" e com emoji na posição 60; renderizar card sem panic.

## CRM-003 — Excluir etapa deixa negócios órfãos invisíveis

- Prioridade: Alta
- Área: Etapas / Métricas
- Confirmado visualmente: Sim
- Confirmado no código: Sim
- Ambiente: janela ≈ 1135 px
- Pré-condição: etapa temporária contendo o negócio de teste
- Passos: 1. criar etapa; 2. mover o card para ela; 3. clicar na lixeira da etapa.
- Comportamento atual: etapa excluída sem confirmação; o card desaparece de todas as colunas, mas o cabeçalho continua "1 negócios"; a sidebar passa a mostrar "Selecione".
- Comportamento esperado: confirmar; mover os negócios para outra etapa (ou bloquear a exclusão); manter contagens coerentes.
- Evidência: capturas do funil antes/depois (colunas Lead 0 / Pós-Venda 0, KPI ainda "1 negócios").
- Arquivo e linhas: `src/ui/kanban.rs` L544-548 (ação direta); `src/app.rs` L5517-5520; `src/archive/crm.rs` L109-114 (sem cascata, sem FK); `src/ui/kanban.rs` L493-496 (filtra por `column_id`, órfãos somem); `src/ui/crm_sidecar.rs` L226-244 (`unwrap_or(0)` pinta 1 segmento).
- Risco para dados: negócio "perdido" (dado existe, mas inacessível); métricas incoerentes.
- Sugestão: diálogo de confirmação com destino dos negócios; coluna "Sem etapa" para órfãos; migração ao carregar.
- Teste de regressão: excluir etapa com negócio → negócio aparece na etapa destino; totais batem.

## CRM-004 — Largura da sidebar volta sozinha ao soltar a borda

- Prioridade: Alta
- Área: Sidebar
- Confirmado visualmente: Sim (durante o arrasto o painel alarga; ao soltar volta para ≈ 320 px)
- Confirmado no código: Sim
- Ambiente: escala ≈ 100%
- Passos: 1. abrir a sidebar; 2. segurar a borda esquerda e arrastar para a esquerda; 3. soltar.
- Comportamento atual: largura final = 320 px.
- Comportamento esperado: manter a largura escolhida (e idealmente lembrá-la ao reabrir).
- Evidência: captura durante o arrasto (painel ≈ 400 px) e após soltar (≈ 320 px).
- Arquivo e linhas: `src/ui/crm_sidecar.rs` L28 (`data.remove::<egui::PanelState>(id)` a cada frame), L25-26 e L30-33 (`default_size(wanted)` fixo em 320); overlay também fixo em L53.
- Risco para dados: nenhum.
- Sugestão: remover L28; guardar largura em estado/configuração (`app.crm_sidecar_width`) e passá-la a `default_size`.
- Teste de regressão: arrastar, soltar, interagir com campo, fechar/reabrir → largura preservada.

## CRM-005 — Valor "1.500,00" vira R$ 0,00 sem aviso

- Prioridade: Alta
- Área: Valor da negociação
- Confirmado visualmente: Sim
- Confirmado no código: Sim; reproduzido isoladamente com 14 entradas
- Passos: 1. sidebar → Valor; 2. digitar `1.500,00`; 3. Salvar.
- Comportamento atual: cabeçalho continua "R$ 0,00"; o campo mantém "1.500,00"; nenhuma mensagem. `100,50` e `1500,00` funcionam.
- Comportamento esperado: aceitar separador de milhar brasileiro ou avisar que o valor é inválido.
- Evidência: capturas (campo "1.500,00" / header R$ 0,00) e (campo "100,50" / header R$ 100,50). Isolado: `1.500,00→0`, `abc→0`, `-50→0`, `R$ 100→0`, `inf`/`99999999999999999999 → R$ 92.233.720.368.547.758,07`, `1e3→R$ 1.000,00`.
- Arquivo e linhas: `src/ui/crm_sidecar.rs` L287-300 (L288 troca vírgula por ponto sem tratar milhar; L289-293 `else { 0 }`); mesma lógica em `src/ui/kanban.rs` L314-315 (`unwrap_or(0.0)`).
- Risco para dados: pode zerar um valor existente ao salvar um texto inválido; valores absurdos aceitos.
- Sugestão: normalizar (remover `.` de milhar, trocar `,` por `.`), rejeitar NaN/inf/negativo/limite máximo com mensagem; não gravar quando inválido.
- Teste de regressão: tabela de entradas (100, 100,50, 1.500,00, 0, vazio, abc, -5, 1e20) com resultado e feedback esperados.

## CRM-006 — Conteúdo da sidebar transborda; etiqueta longa esconde botões

- Prioridade: Alta
- Área: Sidebar
- Confirmado visualmente: Sim
- Confirmado no código: Sim
- Ambiente: sidebar ≈ 320 px
- Passos: 1. abrir a sidebar (já aparece "+ Adicionar" e "✓ Salvo" cortados na borda direita); 2. adicionar a etiqueta `ETIQUETA-MUITO-LONGA-PARA-TESTAR-O-TRANSBORDAMENTO-DA-SIDEBAR-1234567890 🚀`; 3. criar um lembrete.
- Comportamento atual: o corpo da sidebar fica mais largo que o painel; "+ Adicionar", "Amanhã 09h", "+1 semana" cortados; "✓ Salvo" some; o "×" da etiqueta longa fica sob a barra de rolagem (não dá para remover pela interface) e os botões Concluir/Excluir do lembrete ficam parcialmente/totalmente fora da área visível.
- Comportamento esperado: conteúdo sempre dentro do painel, textos truncados/quebrados, botões acessíveis.
- Evidência: capturas da sidebar sem e com a etiqueta longa; zoom do card de lembrete.
- Arquivo e linhas: `src/ui/crm_sidecar.rs` L347-355 (`desired_width(available - 80)` + botão mais largo que 80), L320-337 (chips sem limite de largura), L112-114 (ScrollArea só vertical), L471-485 (linha right-to-left do lembrete), L464 (título sem truncar).
- Risco para dados: etiquetas e lembretes que não podem ser removidos/concluídos pela UI.
- Sugestão: usar `ui.available_width()` corretamente (layout com `Layout::right_to_left` para o botão), limitar chip a `max_width` com truncamento, `ScrollArea::both` ou `Label::truncate()`; limitar o comprimento da etiqueta (ex.: 32).
- Teste de regressão: etiqueta de 80 caracteres e título de lembrete de 200 caracteres a 280 px — todos os botões clicáveis.
- Obs.: etiqueta longa aceitou emoji e foi gravada; para limpar o estado foi usada a rota "Adicionar ao Funil" (CRM-001).

## CRM-007 — "Fechamentos Ganhos" depende de etapa com id `close`

- Prioridade: Alta
- Área: Métricas
- Confirmado visualmente: Sim (o funil atual não tem `close`; o indicador fica sempre R$ 0,00 mesmo com negócio na última etapa)
- Confirmado no código: Sim
- Passos: Funil → Métricas.
- Comportamento atual: R$ 0,00 em "Fechamentos Ganhos".
- Comportamento esperado: definir a etapa "ganho" por flag/ordem (última etapa) e não por id fixo.
- Evidência: captura de Métricas (Pipeline R$ 100,50; Fechamentos R$ 0,00).
- Arquivo e linhas: `src/ui/crm_metrics.rs` L39-42 (comentário diz "last column or close"; implementação só `close`).
- Risco para dados: indicadores comerciais errados (não altera dados).
- Sugestão: campo `is_won` em `CrmColumn` (ou última etapa por `order`).
- Teste: renomear Fechamento, usar id diferente, excluir/restaurar padrões — métrica coerente.

## CRM-008 — Foto real não aparece (ícone genérico)

- Prioridade: Média
- Área: Avatar
- Confirmado visualmente: Sim (lista de conversas e cabeçalho mostram foto; sidebar e card do Kanban mostram ícone de usuário)
- Confirmado no código: Sim
- Arquivo e linhas: `src/ui/crm_sidecar.rs` L145-150 (círculo + `Icon::User`); `src/ui/kanban.rs` L629-633 (idem). O sistema de avatares já existe: `app.avatar(&chat.id)` (`src/app.rs` L2382) e `widgets::avatar/paint_avatar` (usados em `src/ui/conversation.rs` L132-146, L847-848).
- Sugestão: usar `app.avatar` + `widgets::paint_avatar` (iniciais coloridas como fallback), também nos cards.
- Teste: contato com foto → sidebar e card exibem a mesma imagem; sem foto → iniciais.
- Não comparada: tela de informações do contato (não aberta).

## CRM-009 — Cabeçalho de coluna e card com texto longo sobrepõem controles

- Prioridade: Média
- Área: Kanban
- Confirmado visualmente: Sim
- Passos: criar etapa "TESTE CRM etapa temporária" → o nome sobrepõe o contador e o botão "+"; a lixeira fica colada ao contador. No card, "💬 Conversar" aparece cortado ("Conversai") ao lado de "Mover etapa".
- Arquivo e linhas: `src/ui/kanban.rs` L527-560 (título sem truncar, L531), L636 (título do card), L687-717 (botão + combo de 110 px na mesma linha).
- Sugestão: `Label::truncate()`/largura máxima, reservar espaço para controles, permitir quebra de linha no rodapé do card.
- Teste: etapa com 60 caracteres e contato com nome longo — nenhum controle sobreposto.

## CRM-010 — Diálogos de arquivo bloqueiam a interface e abrem em outro monitor

- Prioridade: Média
- Área: Backup
- Confirmado visualmente: Sim
- Observado: ao clicar "Exportar Backup JSON", a janela do app ficou congelada/inativa e o diálogo "Salvar backup do CRM" abriu no monitor principal, não no monitor onde o app estava; `Esc` não fechou nada no app. O diálogo abriu na pasta Downloads.
- Arquivo e linhas: `src/ui/kanban.rs` L137-155 (`rfd::FileDialog::…save_file()/pick_file()` síncrono dentro do frame, sem `set_parent`).
- Risco para dados: nenhum direto; o usuário pode achar que o app travou.
- Sugestão: `rfd::AsyncFileDialog` ou thread + canal; `set_parent(&window)`.
- Teste: exportar com app em monitor secundário — diálogo aparece sobre o app e a UI continua respondendo.

## CRM-011 — Backup JSON em texto puro

- Prioridade: Média
- Área: Backup / Privacidade
- Confirmado: arquivo exportado lido (JSON legível com `chat_id` = telefone, notas e etiquetas), enquanto a sidebar rotula as notas como "Criptografadas" (o banco é SQLCipher, mas o backup não).
- Arquivo e linhas: `src/archive/crm.rs` L247-263; `src/backend/worker.rs` L5892-5901.
- Sugestão: avisar na UI antes de exportar; opção de backup cifrado com senha; local padrão fora de Downloads.

## CRM-012 — Nova etapa entra no meio; ID de etapa por segundo

- Prioridade: Média
- Área: Etapas
- Confirmado visualmente: Sim (a etapa criada apareceu entre "Lead" e "Pós-Venda")
- Confirmado no código: Sim; colisão de ID demonstrada isoladamente (`col_{timestamp em segundos}`)
- Arquivo e linhas: `src/ui/kanban.rs` L397 (id), L400 (`order = crm_columns.len()` — com ordens 1 e 4 existentes resulta 2, antes de 4); `src/app.rs` L5508-5515; `src/archive/crm.rs` L99-107 (upsert por id: duas etapas no mesmo segundo → a segunda sobrescreve a primeira).
- Sugestão: `order = max(order)+1`; id com UUID/contador.
- Teste: criar 2 etapas no mesmo segundo; criar etapa com ordens existentes 1 e 4.

---

# 2. Bugs prováveis encontrados no código (não reproduzidos na tela)

## CRM-013 — IDs temporários globais vazam rascunhos entre contatos

- Prioridade: Alta
- Área: Persistência / Sidebar
- Confirmado visualmente: Não (restrição: só um contato autorizado — não troquei de conversa)
- Confirmado no código: Sim
- Arquivo e linhas: `src/ui/crm_sidecar.rs` L267 (`sidecar_deal_value_input`), L344 (`sidecar_new_tag_input`), L391 (`sidecar_internal_notes`), L494 (`sidecar_new_followup_title`), L204 (`sidecar_stage_selector`); ScrollArea L112 sem `id_salt` por contato.
- Cenário: editar a nota do contato A; abrir a sidebar do contato B → o campo mostra a nota de A (valor em `data.get_temp`); se o usuário focar e sair do campo, `notes_draft != deal.notes` (L404) grava o texto de A no contato B.
- Risco para dados: **vazamento de informação privada entre contatos e sobrescrita de notas.**
- Sugestão: incluir `chat.id` em todos os `egui::Id` (`Id::new(("crm_notes", &chat.id))`).
- Teste: dois contatos, nota diferente em cada, alternar com sidebar aberta; rascunhos não vazam.

## CRM-014 — Falhas de gravação são ignoradas

- Prioridade: Alta
- Área: Persistência
- Arquivo e linhas: `src/backend/worker.rs` L5864-5890 (`let _ = self.archive.upsert_crm_*/delete_crm_*/complete…/snooze…`), L1198-1213 (`emit_crm_data` só `log::warn`); `src/app.rs` L5521-5524 (`SaveCrmDeal` atualiza o estado local antes da confirmação do banco).
- Risco: a UI mostra sucesso (ou depois "reverte" sem explicação) quando o banco rejeita.
- Sugestão: propagar `Event::Error` (como já é feito em export/import, L5892-5915) e só confirmar após o `Ok`.
- Teste: forçar falha de escrita (banco somente leitura) e verificar mensagem ao usuário.

## CRM-015 — Notas só gravam ao perder foco; "✓ Salvo" é fixo

- Prioridade: Média
- Confirmado visualmente: parcialmente — "✓ Salvo" está sempre visível, inclusive com texto digitado e ainda não gravado; fechar a sidebar com clique **preservou** a nota (o clique tira o foco antes).
- Confirmado no código: Sim — `src/ui/crm_sidecar.rs` L385 (rótulo estático), L404-409 (`lost_focus`).
- Cenários **não testados**: fechar o app/janela com o campo em foco, atalho de teclado, queda de energia, troca de contato por teclado.
- Sugestão: estado "Salvando…/Salvo" real; gravar com debounce (≈ 800 ms) e ao fechar painel/app.

## CRM-016 — Campo de valor mantém texto antigo

- Prioridade: Média
- Código: `src/ui/crm_sidecar.rs` L267-274 — após o primeiro uso o texto vem de `get_temp` e nunca é recarregado de `deal.value_cents`. Depois de CRM-001 (valor zerado por fora) o campo continuaria mostrando "100,50" e `Salvar` regravaria o valor antigo. Também mostra `1500.00` (ponto) enquanto o resto da UI usa vírgula (L270).
- Não verificado visualmente.

## CRM-017 — "Restaurar 5 Etapas Padrão" conflita com etapas personalizadas

- Prioridade: Média
- Código: `src/ui/kanban.rs` L131-135 chama `SaveCrmColumn` com ids `lead/qual/prop/close/post` → sobrescreve título/cor/ordem de qualquer etapa padrão editada; coexiste com etapas personalizadas (aqui existiria "Lead" `col_…` e "Novos Contatos" `lead`; ordens duplicadas, ex.: `col_…`=1 e `qual`=1). Não testado para não alterar etapas reais.
- Sugestão: "Restaurar" apenas adiciona as que faltam, com ordem após a última, e pergunta antes de sobrescrever.

## CRM-018 — Importação de backup sem transação nem validação

- Prioridade: Média
- Código: `src/archive/crm.rs` L266-277 (loops com `?`: falha no meio deixa importação parcial; não valida `column_id` existente; sempre mescla/sobrescreve). Exportação testada com sucesso; **importação não testada** (menu inconclusivo, ver seção 5).

## CRM-019 — Métricas: KPIs de largura fixa e percentuais com órfãos

- Prioridade: Média
- Código: `src/ui/crm_metrics.rs` L124 (`set_width(120.0)` × 5 cards + margens + espaçamentos ≈ 740 px) dentro de diálogo fixo de 680 px (`src/ui/dialogs.rs` L58) e sem `ScrollArea` horizontal (L49-56). Na janela de ~900 px do teste os cards couberam; **não testado em janela estreita**. Percentuais usam `total_deals` incluindo órfãos (L74-78), logo não somam 100%; Ticket Médio inclui negócios de R$ 0 (L33-37).

## CRM-020 — Barra superior do Funil em linha única

- Prioridade: Média
- Código: `src/ui/kanban.rs` L101-172 — título, KPI e 5 controles numa linha; diálogo clampado em 320–1400 px (`src/ui/dialogs.rs` L59). Em janela de ~900 px couberam; **janela estreita não testada**.

## CRM-021 — Follow-ups: adiar e ID

- Prioridade: Baixa
- Código: `src/ui/crm_sidecar.rs` L481 (`+1d` = `remind_at + 86400`, não "agora + 1 dia": lembrete atrasado há 3 dias continua atrasado), L539 e L594-596 (`fu_{now}_{now%10000}`: dois lembretes no mesmo segundo colidem e o segundo sobrescreve o primeiro via `ON CONFLICT`, `src/archive/crm.rs` L204-223).
- O "+1d" **funcionou** no teste (07/10 10:16 → 08/10 10:16) por não estar atrasado.

---

# 3. Melhorias de interface

- UI-01: "Taxa de Conversão por Etapa" mostra distribuição, não conversão (`crm_metrics.rs` L59).
- UI-02: "1 contatos" / "1 negócios" sem plural correto (`crm_metrics.rs` L96; `kanban.rs` L111).
- UI-03: ícone de alerta no título de Métricas (`crm_metrics.rs` L16); emojis dos botões ("💬", "📊", "💾", "🔄") renderizam como quadrados pequenos.
- UI-04: abrir Métricas fecha o Funil (substitui o diálogo; X leva ao chat, não ao Funil); `Esc` não fecha os diálogos.
- UI-05: sem feedback para etiqueta duplicada (é ignorada em silêncio, `crm_sidecar.rs` L357-366) e para valor inválido.
- UI-06: sem limite de tamanho para nome de etapa, etiqueta e título de lembrete.
- UI-07: card de lembrete concluído não permite reabrir; só excluir.
- UI-08: botões de ação sem confirmação em operações destrutivas (excluir etapa/lembrete).

---

# 4. Validações que passaram

- Sidebar abre/fecha, acoplada à direita da conversa, sem cobrir o campo de mensagem nem o cabeçalho; atalho "Ver no Funil" abre o Funil e fechar volta para a conversa com a sidebar aberta.
- Trocar etapa na sidebar atualiza imediatamente nome, cor do ponto e barra de progresso; o Funil mostra o contato na coluna correta (exatamente uma) e a contagem/total da coluna.
- Valor `100,50` salvo e exibido como R$ 100,50 (sidebar, card, coluna, Pipeline Total, Ticket Médio).
- Etiquetas: adicionar (com espaço, acento, emoji), duplicada ignorada, chips no card do Kanban.
- Nota com acentos e emoji (✅ çãõ 🚀, 67 bytes) gravada, preservada ao fechar a sidebar logo após digitar, e com trecho exibido no card sem erro.
- Follow-up: criar com "+1 hora", aparece na hora; "+1d" atualiza data; "Concluir" muda para o estado concluído imediatamente; Métricas ignoram lembretes concluídos.
- Funil: "+ Novo Negócio" (pesquisa de contato, seleção, etapa inicial), "+ Nova Etapa", "Mover etapa" no card, "Métricas", "Fechar" e "Exportar Backup JSON" (arquivo válido) funcionaram.
- Nenhuma mensagem enviada; nenhuma conversa de outro contato aberta.

# 5. Casos não reproduzidos / não testados

- **Persistência após fechar e reabrir o app** (Parte 12 do roteiro): não testada — a conexão caiu antes.
- Janela estreita/média/larga/maximizada, modo overlay da sidebar (< 640 px) e escalas 125%/150%: não testados (apenas a janela foi movida de lugar).
- Importar Backup JSON: o menu "Backup / Opções" parou de abrir após o primeiro uso (5 cliques seguidos sem efeito, coincidindo com a janela do Claude em primeiro plano); considerado **inconclusivo**, não atribuído ao app.
- Panic de CRM-002, vazamento de rascunho de CRM-013, perda de nota ao fechar o app (CRM-015), "Restaurar 5 Etapas Padrão" (CRM-017): não executados de propósito, para não derrubar o app em uso, não tocar em outros contatos e não alterar etapas reais.
- Busca/limpar busca, botão "Conversar" (não clicado), foto na tela de informações do contato, valores 0/vazio/negativo/muito grande diretamente no app (cobertos só pela reprodução isolada), etapas personalizadas além da temporária, muitas etapas, cores inválidas.

# 6. Ordem recomendada de correção

1. CRM-001 (sobrescrita de negócio) e CRM-002 (panic) — perdas de dados/queda.
2. CRM-013 (vazamento entre contatos) e CRM-014 (erros de banco ignorados).
3. CRM-003 (etapa excluída com negócios) + CRM-012 (ordem/ID de etapas) + CRM-017 (restaurar padrão).
4. CRM-005 (valor) e CRM-016 (campo de valor desatualizado).
5. CRM-004 (largura da sidebar) e CRM-006 (transbordo/botões inacessíveis).
6. CRM-007 (Fechamentos Ganhos) e CRM-019 (métricas).
7. CRM-015 (salvar nota com debounce / "Salvo" real).
8. CRM-008 (avatar) e CRM-009 (textos longos no Kanban).
9. CRM-010, CRM-011, CRM-018 (backup/arquivos).
10. CRM-020, CRM-021 e melhorias de interface.
