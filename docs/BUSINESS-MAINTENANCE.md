# Manual de manutenção e releases do ZapFast Business

Este documento permite que outro desenvolvedor ou agente, inclusive o Google
Antigravity, continue o fork sem misturar a instalação Business com o ZapFast
original. Leia também `AGENTS.md`, `BUSINESS.md`, `README.md` e `PACKAGING.md`
antes de alterar código.

## Repositórios, branch e fontes consultadas

- Fork: `https://github.com/DLangkamer/zapfast-business`
- Upstream: `https://github.com/crmne/zapfast`
- Branch mantida: `business/windows-isolation`
- Releases Business: `https://github.com/DLangkamer/zapfast-business/releases`
- Releases upstream: `https://github.com/crmne/zapfast/releases`
- Protocolo WhatsApp: revisão fixada de `oxidezap/whatsapp-rust` em `Cargo.toml`
- Framework compartilhado: revisão/tag de `crmne/fastframe` em `Cargo.toml`

As consultas de versão são feitas em três lugares: tags e releases do upstream,
histórico Git entre a base já integrada e a tag nova, e notas em
`packaging/release-notes/`. Nunca copie um binário do upstream para a release
Business: o binário precisa ser compilado deste fork.

## Preparação do checkout

```powershell
git clone https://github.com/DLangkamer/zapfast-business.git
cd zapfast-business
git checkout business/windows-isolation
git remote add upstream https://github.com/crmne/zapfast.git
git fetch origin --tags --prune
git fetch upstream --tags --prune
git status --short
```

No Windows, instale Rust conforme `rust-toolchain.toml`, Visual Studio Build
Tools com C++ e Windows SDK, CMake, Perl completo e Inno Setup 6. Caminhos muito
longos podem quebrar a compilação vendorizada do OpenSSL. Nesse caso use um
diretório de build curto:

```powershell
$env:CARGO_TARGET_DIR = 'C:\build\zapfast-business'
```

Não coloque senhas, tokens ou chaves de assinatura em arquivos, commits, logs
ou prompts. A chave privada da atualização existe apenas como secret do GitHub
`ZAPFAST_BUSINESS_UPDATE_SIGNING_KEY`. O repositório contém somente a chave
pública em `assets/business-update-public-key.hex`.

## Como verificar uma nova versão upstream

```powershell
git fetch upstream --tags --prune
git tag --list 'v*' --sort=-version:refname | Select-Object -First 10
git log --oneline --decorate --no-merges v0.19.0..upstream/main
git diff --stat v0.19.0..upstream/main
git diff --name-status v0.19.0..upstream/main
```

Substitua `v0.19.0` pela última tag upstream já integrada. Leia a página da
nova release e suas notas. Depois examine especialmente mudanças em:

- `src/main.rs`, `src/paths.rs`, `src/single_instance.rs`, `src/notify.rs`;
- `src/archive/encryption.rs`, `src/updates.rs`, `src/settings.rs`;
- `Cargo.toml`, `Cargo.lock`, `build.rs`;
- `.github/workflows/`, `native-packages.yaml` e `packaging/windows/`;
- modelos, comandos/eventos do backend, worker e telas afetadas pelas funções
  exclusivas do Business.

## Integração do upstream

Crie primeiro uma branch de segurança ou confirme que a branch Business está
limpa. Faça o merge da tag, preservando a relação histórica com o upstream:

```powershell
git checkout business/windows-isolation
git pull --ff-only origin business/windows-isolation
git merge --no-commit --no-ff vNOVA.VERSAO.UPSTREAM
git diff --name-only --diff-filter=U
rg -n '^(<<<<<<<|=======|>>>>>>>)' . -g '!target'
```

Resolva conflitos arquivo por arquivo. Use a versão upstream como base nos
arquivos de funcionalidades gerais e reaplique o menor patch Business possível.
Nunca escolha automaticamente “theirs” nos arquivos de identidade e isolamento.

Após resolver, confirme que não restou marcador de conflito e revise tanto o
diff staged quanto o unstaged:

```powershell
rg -n '^(<<<<<<<|=======|>>>>>>>)' . -g '!target'
git diff --name-only --diff-filter=U
git diff --check
git diff --cached --check
git status
```

## Contrato de isolamento que toda atualização deve preservar

Os valores canônicos ficam em `src/identity.rs`:

- nome: `ZapFast Business`;
- slug/executável: `zapfast-business` / `zapfast-business.exe`;
- AppUserModelID: `io.github.DLangkamer.ZapFastBusiness`;
- serviço de credencial: `io.github.DLangkamer.ZapFastBusiness.archive`.

Audite os pontos abaixo em toda integração:

1. `src/paths.rs` usa `zapfast-business` para configuração, estado, cache e
   runtime. `adopt_previous_names()` deve continuar sem migrar `zapfast`,
   `fastsapp` ou `fastwhatsapp`. A migração interna do layout Business antigo
   para `accounts/1` pode permanecer.
2. `src/single_instance.rs` usa um slot Business e nunca consulta nem abre a
   porta fixa legada do ZapFast original.
3. `src/archive/encryption.rs` usa `identity::KEYRING_SERVICE` em todas as
   operações de criar, copiar e apagar chaves.
4. Janela, tray, notificações, tela de login, diálogo de erro e inicialização
   automática exibem `ZapFast Business` e usam o AppUserModelID Business.
5. `src/updates.rs` consulta somente `DLangkamer/zapfast-business`, exige o
   manifesto assinado e aceita somente assets com slug Business.
6. `packaging/windows/zapfast.iss` mantém AppId, pasta
   `%LOCALAPPDATA%\Programs\ZapFast Business`, atalhos, Run key, filtro de
   fechamento e desinstalador próprios.
7. O workflow usado é `.github/workflows/release.yml`. Não publique usando o
   empacotamento multiplataforma original enquanto ele ainda referenciar
   `crmne/zapfast`.

Buscas úteis para encontrar regressões:

```powershell
rg -n 'rocks\.zapfast\.ZapFast|me\.paolino\.fastsapp|fastsapp:' src
rg -n 'Self::of\("zapfast"\)|const NAME: &str = "fastsapp"' src
rg -n 'crmne/zapfast|DLangkamer/zapfast-business' src .github packaging
rg -n '"ZapFast"|"zapfast"' src build.rs packaging/windows
```

Nem toda ocorrência do nome original é erro: créditos, links ao upstream,
notas históricas e identificadores de stickers podem continuar. Cada ocorrência
em caminhos, instância, credenciais, updater, janela ou sistema operacional deve
ser corrigida.

## Arquitetura para novas funções

A interface adiciona `Action` em `src/model.rs`; `App::apply` em `src/app.rs`
valida estado e envia um `Command`; o worker em `src/backend/worker.rs` é o
único dono do protocolo e do `Archive`; resultados voltam por `Event`. Preserve
essa direção. Não acesse diretamente rede ou SQLite a partir da interface.

O ZapFast 0.19 possui várias contas. Estado associado a uma conta deve ficar em
`src/account.rs`, e cada conta tem backend, arquivo criptografado e diretórios
próprios. Estado puramente visual da janela pode ficar em `App`.

Agendamentos ficam em `src/archive/scheduled.rs`. A tabela é SQLCipher porque
faz parte de `archive.db`. As migrações de colunas são registradas no array
`MIGRATIONS` de `src/archive.rs`. Atualmente há texto e voz; a central permite
listar, editar horário/texto e cancelar por conta. A voz é guardada como amostras
`f32` codificadas em bytes e só passa pelo envio normal quando vence. Não crie
um segundo caminho de envio que ignore as validações do worker.

O calendário visual interativo fica em `src/ui/dialogs.rs` (`render_calendar_picker`),
oferecendo seleção visual de dia, mês, hora e minutos com atalhos de tempo (+15m, +30m,
+1h, +3h, Amanhã 09:00). Indicadores e botões com contadores de agendamentos
(`scheduled_count`) ficam disponíveis diretamente na lista de conversas (`src/ui/chats.rs`)
e no cabeçalho da conversa aberta (`src/ui/conversation.rs`), permitindo acesso imediato
e reagendamento visual.

O módulo de Disparo em Massa e Listas de Transmissão fica em `src/ui/bulk_dispatch.rs`:
- **Colar Números**: suporta listas coladas separadas por linha, vírgula ou ponto-e-vírgula.
  O utilitário `parse_phone_number` sanitiza pontuações, valida o formato internacional e
  injeta automaticamente o DDI 55 para números brasileiros sem prefixo de país.
- **Listas de Transmissão Segmentadas**: criação e edição de listas de contatos e grupos,
  persistidas na tabela SQLCipher `business_broadcast_lists` (`src/archive/broadcast_lists.rs`).
- **Áudio PTT Autêntico**: suporte a arquivos de áudio (.mp3, .ogg, .wav, .m4a) convertidos
  nativamente em mono 48 kHz com waveform de 64 barras, entregues como notas de voz genuínas.
- **Envio Intercalado Anti-Bloqueio**: slider e atalhos rápidos de intervalo (5s a 2 min)
  com estimativa dinâmica de tempo total. No modo agendado, o escalonamento é registrado no
  banco de dados com timestamps espaçados (`due_at = schedule_at + i * interval_seconds`).

Nunca use conversas reais como fixture. Testes devem usar bancos temporários,
chats e áudios sintéticos. Não leia nem copie conteúdo do perfil instalado.

## Versão, documentação e commit

Uma versão Business baseada em upstream `X.Y.Z` deve usar um número Business
posterior e ainda livre, por exemplo `X.Y.(Z+1)`. Atualize:

- `package.version` em `Cargo.toml`;
- `Cargo.lock`, regenerado pelo Cargo;
- `packaging/release-notes/vVERSAO.md`;
- referências de versão e base em `BUSINESS.md` e README quando aplicável.

Use commits pequenos durante desenvolvimento. Para a integração final, um merge
commit pode reunir a tag upstream e as adaptações Business. Exemplos:

```powershell
git add src/model.rs src/backend.rs src/backend/worker.rs
git commit -m "Add scheduled message management"
git add -A
git commit -m "Merge ZapFast X.Y.Z and preserve Business isolation"
```

Mensagens devem dizer o comportamento entregue. Não altere autoria, licença,
`LICENSE`, créditos ou avisos de terceiros.

## Verificações locais

```powershell
cargo fmt --all --check
cargo check --locked --all-targets
cargo test --locked archive::scheduled::tests --lib
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Use um target curto se o OpenSSL falhar com `C1083` ou caminho inválido. O
OpenSSL vendorizado exige Perl completo; o Perl mínimo incluído no Git for
Windows pode não conter todos os módulos.

Antes de publicar, execute novamente as buscas de isolamento, `git diff
--check`, confirme `git status` limpo e inspecione `git log -1`.

## Build e instalador local

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc
iscc /DVersion=VERSAO /DNumericVersion=VERSAO /DArch=x86_64 `
  /DBinary="$PWD\target\x86_64-pc-windows-msvc\release\zapfast-business.exe" `
  /DOutputDir="$PWD\dist" packaging\windows\zapfast.iss
```

O executável esperado é
`target\x86_64-pc-windows-msvc\release\zapfast-business.exe`; o instalador é
`dist\zapfast-business-vVERSAO-x86_64-pc-windows-msvc-setup.exe`.

Não instale por cima do ZapFast pessoal. Para validar localmente, confira a
pasta de destino, nomes dos atalhos, processo, registro de inicialização e
desinstalador. Abra ambos e verifique somente processo/janela/diretórios; não
abra conversas. A vinculação, o envio real, notificações reais e recursos que
dependem do celular precisam de validação manual do usuário.

## Publicação no GitHub

```powershell
git push origin business/windows-isolation
git tag -a vVERSAO -m "ZapFast Business vVERSAO"
git push origin vVERSAO
```

O push da tag dispara `.github/workflows/release.yml`. O workflow:

1. confirma que a tag pertence à branch Business, que a versão bate com
   `Cargo.toml` e que existem notas da release;
2. compila o Windows x64 em modo release;
3. cria ZIP portátil e instalador Inno Setup;
4. gera checksums e manifesto assinados com o secret;
5. publica a GitHub Release como latest.

Acompanhe sem baixar ou revelar secrets:

```powershell
gh run list --repo DLangkamer/zapfast-business --workflow release.yml --limit 5
gh run watch ID_DO_RUN --repo DLangkamer/zapfast-business --exit-status
gh release view vVERSAO --repo DLangkamer/zapfast-business
```

Se o workflow falhar, corrija a branch, crie novo commit e mova a tag somente
antes de haver usuários dessa release. Depois de publicada, prefira um novo
número de versão. Nunca substitua silenciosamente assets já distribuídos.

## Teste do atualizador

1. Mantenha instalada uma versão Business anterior.
2. Confirme que a nova release terminou e contém instalador, ZIP, checksums e
   assinatura.
3. No aplicativo anterior, abra **Settings → About → Check for updates now**.
4. Confirme que ele encontra exatamente a release do fork, baixa o instalador
   Business, reinicia na pasta Business e preserva a conta vinculada.
5. Confirme que o ZapFast pessoal continua instalado e pode abrir ao mesmo
   tempo.

Uma instalação já na versão nova deve informar que está atualizada. Teste
manual de envio agendado exige uma conta vinculada e autorização explícita do
usuário; a automação de manutenção não deve enviar mensagens.

## Checklist para entregar uma atualização

- Upstream e notas lidos; tag exata registrada.
- Conflitos resolvidos sem perder recursos Business.
- Versão e release notes atualizadas.
- Isolamento de diretórios, instância, credenciais, identidade e updater
  auditado.
- Formatação, check e testes relevantes aprovados.
- Branch enviada ao fork e commit/link informados.
- Tag enviada e workflow concluído.
- Release contém instalador assinado e está marcada como latest.
- Atualizador de uma versão anterior encontra a nova release.
- Limitações que dependem do celular foram informadas ao usuário.

## Prompt recomendado para outro agente

> Trabalhe em `business/windows-isolation` do fork
> `DLangkamer/zapfast-business`. Leia `AGENTS.md`, `BUSINESS.md` e
> `docs/BUSINESS-MAINTENANCE.md`. Verifique a release mais recente de
> `crmne/zapfast`, faça merge da tag preservando todo o contrato de isolamento,
> mantenha licenças e créditos, implemente a alteração pedida pelo fluxo
> Action → Command → worker/archive → Event, use apenas fixtures sintéticas,
> execute formatação/check/testes, atualize versão e notas, publique branch e
> tag e acompanhe o workflow até a release assinada. Não leia conversas reais,
> não envie mensagens e nunca exponha secrets.
