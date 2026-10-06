# 🐳 Guia de Instalação e Uso: ZapFast Server (Docker) + Desktop Client

Este guia explica como executar o **ZapFast Server** em container Docker e conectar o **ZapFast Desktop** com isolamento de contas, permissões de atendimento e anti-delete centralizado.

---

## 1. Visão Geral da Arquitetura

O sistema é composto por duas partes integradas:

1. **Servidor Docker (`zapfast-server`)**:
   - Roda em segundo plano (headless) em qualquer servidor Linux, Windows ou nuvem com Docker.
   - **Web Dashboard (Porta 8080)**: Interface web para gerenciar contas de WhatsApp, gerar QR Codes e criar usuários.
   - **Sistema de Permissões Granular**: Permite associar quais atendentes/usuários têm acesso a quais números de WhatsApp.
   - **Anti-Delete Permanente**: Armazena todas as mensagens no banco SQLCipher; se o remetente apagar uma mensagem para todos, o servidor preserva o texto e a mídia original com a marcação `🚫 [Apagada]`.
   - **Auto-Discovery na Rede Local**: Beacon UDP na porta `47120` para detecção automática instantânea na rede local.

2. **Aplicativo Desktop (`zapfast-business`)**:
   - Funciona em **Modo Local Standalone** (uso direto tradicional) OU conectado ao **Servidor ZapFast (Docker)**.
   - Botão **"🔍 Buscar na Rede Local"** para localizar o servidor na rede sem precisar digitar o IP.
   - Login com usuário e senha do servidor, carregando apenas as contas permitidas para aquele atendente.

---

## 2. Como Subir o Servidor no Docker

### Pré-requisitos
- Docker e Docker Compose instalados.

### Passo 1: Iniciar com Docker Compose

Na pasta do projeto, execute:

```bash
docker compose up -d --build
```

O container irá compilar o binário otimizado `zapfast-server` e inicializar os seguintes serviços:
- **Painel Web & API**: `http://localhost:8080`
- **Descoberta na Rede Local (UDP)**: Porta `47120/udp`
- **Volume persistente**: `zapfast_server_data` (armazena sessões, bancos de dados e mídias)

### Passo 2: Acessar o Painel Web Administrativo

1. Abra seu navegador em `http://localhost:8080` (ou `http://IP_DO_SERVIDOR:8080`).
2. Faça login com as credenciais padrão do administrador:
   - **Usuário**: `admin`
   - **Senha**: `admin123`

---

## 3. Configurando Contas e Usuários no Painel Web

### 📱 Conectar Contas de WhatsApp Separadamente
1. No menu superior, clique em **📱 Contas WhatsApp**.
2. Clique no botão **`+ Conectar Nova Conta`**.
3. Digite um identificador (ex: `vendas_01`, `suporte_sac`) e um nome de exibição.
4. O servidor iniciará a instância e exibirá o **QR Code na tela**.
5. No celular: abra o WhatsApp > **Aparelhos conectados** > **Conectar um aparelho** e aponte a câmera para o QR Code.
6. A conta ficará com status **✔ Conectado**.

### 👥 Criar Usuários e Definir Permissões
1. No menu superior, clique em **👥 Usuários & Permissões**.
2. Clique em **`+ Criar Novo Usuário`**.
3. Defina o login (ex: `atendente_joao`), a senha e o tipo de perfil:
   - **Operador**: Acesso restrito apenas aos WhatsApps autorizados.
   - **Administrador**: Acesso total a todas as contas.
4. Na tabela de usuários, clique no botão **Permissões** ao lado do operador.
5. Marque as contas de WhatsApp que aquele atendente poderá ver e atender, e clique em **Salvar Permissões**.

---

## 4. Conectando o Aplicativo Desktop ao Servidor

1. Abra o aplicativo **ZapFast** no seu computador.
2. Na tela inicial ou no seletor de contas, clique em:
   **`🌐 Conectar a Servidor ZapFast (Docker / LAN)…`**
3. Na janela que se abrir:
   - Clique em **`🔍 Buscar Servidores na Rede Local`** para detectar o servidor Docker automaticamente na rede, ou digite o endereço manualmente (ex: `192.168.1.100:8080`).
   - Insira seu **Usuário** e **Senha** cadastrados no servidor.
   - Clique em **Conectar**.
4. O aplicativo sincronizará com o Docker e listará as contas atribuídas a você no switcher de contas, com todas as mensagens, áudios, mídias e o anti-delete funcionando perfeitamente!

---

## 5. Estrutura dos Arquivos Criados

- `Dockerfile`: Multi-stage build para o servidor `zapfast-server`.
- `docker-compose.yml`: Definição dos containers, volumes e portas.
- `src/bin/server.rs`: Ponto de entrada do executável headless do servidor.
- `src/server/db.rs`: Banco de dados `server.db` com controle de usuários, hashes de senha e permissões.
- `src/server/manager.rs`: Gerenciador multi-contas das conexões do WhatsApp.
- `src/server/web_assets.rs`: Painel web HTML5/CSS glassmorphism dark embutido no binário.
- `src/server/http.rs`: Roteador REST API e servidor HTTP.
- `src/server/ws.rs`: Gateway WebSocket em tempo real para Desktop e Web.
- `src/server/discovery.rs`: Responder UDP de descoberta automática na rede local.
- `src/remote/discovery.rs`: Scanner UDP do Desktop para localizar servidores na LAN.
- `src/remote/client.rs`: Cliente WebSocket do Desktop para comunicação com o servidor.
- `src/ui/server_login.rs`: Interface gráfica egui do Desktop para conexão ao servidor.
