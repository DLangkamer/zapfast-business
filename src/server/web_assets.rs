//! Embedded Web Assets for ZapFast Server Admin Dashboard.

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ZapFast Server | Painel de Controle</title>
  <style>
    :root {
      --bg: #0d1117;
      --card-bg: rgba(22, 27, 34, 0.85);
      --card-border: rgba(255, 255, 255, 0.1);
      --accent: #25d366;
      --accent-hover: #1ebd5a;
      --accent-glow: rgba(37, 211, 102, 0.25);
      --danger: #f85149;
      --warning: #d29922;
      --text: #e6edf3;
      --text-muted: #8b949e;
      --input-bg: rgba(13, 17, 23, 0.9);
      --radius: 12px;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }
    body { background-color: var(--bg); color: var(--text); min-height: 100vh; display: flex; flex-direction: column; }
    
    /* Login Overlay */
    #login-overlay {
      position: fixed; inset: 0; background: rgba(13, 17, 23, 0.95); backdrop-filter: blur(10px);
      display: flex; align-items: center; justify-content: center; z-index: 1000;
    }
    .login-box {
      background: var(--card-bg); border: 1px solid var(--card-border); border-radius: var(--radius);
      padding: 36px; width: 100%; max-width: 400px; box-shadow: 0 10px 40px rgba(0,0,0,0.5); text-align: center;
    }
    .login-box h1 { font-size: 24px; margin-bottom: 8px; color: var(--accent); }
    .login-box p { font-size: 14px; color: var(--text-muted); margin-bottom: 24px; }
    
    /* Inputs & Buttons */
    .form-group { margin-bottom: 16px; text-align: left; }
    .form-group label { display: block; font-size: 13px; font-weight: 600; margin-bottom: 6px; color: var(--text-muted); }
    input[type="text"], input[type="password"], select {
      width: 100%; padding: 12px 14px; background: var(--input-bg); border: 1px solid var(--card-border);
      border-radius: 8px; color: var(--text); font-size: 14px; outline: none; transition: 0.2s border;
    }
    input[type="text"]:focus, input[type="password"]:focus, select:focus {
      border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-glow);
    }
    .btn {
      display: inline-flex; align-items: center; justify-content: center; gap: 8px;
      padding: 10px 20px; border-radius: 8px; font-size: 14px; font-weight: 600; cursor: pointer;
      border: none; transition: 0.2s all; text-decoration: none;
    }
    .btn-primary { background: var(--accent); color: #000; }
    .btn-primary:hover { background: var(--accent-hover); box-shadow: 0 0 15px var(--accent-glow); }
    .btn-danger { background: rgba(248, 81, 73, 0.15); color: var(--danger); border: 1px solid rgba(248, 81, 73, 0.4); }
    .btn-danger:hover { background: var(--danger); color: #fff; }
    .btn-secondary { background: rgba(255, 255, 255, 0.08); color: var(--text); }
    .btn-secondary:hover { background: rgba(255, 255, 255, 0.15); }
    .btn-block { width: 100%; padding: 12px; }

    /* Layout */
    header {
      background: var(--card-bg); border-bottom: 1px solid var(--card-border);
      padding: 16px 32px; display: flex; align-items: center; justify-content: space-between;
    }
    .logo-area { display: flex; align-items: center; gap: 12px; font-size: 20px; font-weight: 700; color: var(--text); }
    .logo-dot { width: 12px; height: 12px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 8px var(--accent); }
    .header-user { display: flex; align-items: center; gap: 16px; font-size: 14px; color: var(--text-muted); }

    .nav-tabs {
      display: flex; gap: 12px; background: rgba(22, 27, 34, 0.5); padding: 8px 32px;
      border-bottom: 1px solid var(--card-border);
    }
    .nav-tab {
      padding: 10px 18px; border-radius: 8px; font-size: 14px; font-weight: 600;
      color: var(--text-muted); cursor: pointer; border: none; background: transparent; transition: 0.2s all;
    }
    .nav-tab.active { background: rgba(37, 211, 102, 0.15); color: var(--accent); }

    main { flex: 1; padding: 32px; max-width: 1200px; margin: 0 auto; width: 100%; }
    .section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px; }
    .section-title h2 { font-size: 20px; font-weight: 600; margin-bottom: 4px; }
    .section-title p { font-size: 13px; color: var(--text-muted); }

    /* Cards Grid */
    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 20px; }
    .card {
      background: var(--card-bg); border: 1px solid var(--card-border); border-radius: var(--radius);
      padding: 20px; display: flex; flex-direction: column; gap: 14px;
    }
    .card-header { display: flex; align-items: center; justify-content: space-between; }
    .card-title { font-size: 16px; font-weight: 600; }
    .status-badge {
      display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; border-radius: 12px;
      font-size: 12px; font-weight: 600;
    }
    .status-connected { background: rgba(37, 211, 102, 0.15); color: var(--accent); }
    .status-unlinked { background: rgba(210, 153, 34, 0.15); color: var(--warning); }
    .status-error { background: rgba(248, 81, 73, 0.15); color: var(--danger); }
    .qr-container {
      background: #fff; padding: 12px; border-radius: 8px; align-self: center; margin: 10px 0;
      display: flex; flex-direction: column; align-items: center; justify-content: center;
      min-width: 200px; min-height: 200px;
    }
    .qr-container canvas { width: 180px; height: 180px; }
    .qr-instruction { font-size: 12px; color: #333; text-align: center; margin-top: 6px; }

    /* Table */
    .table-container {
      background: var(--card-bg); border: 1px solid var(--card-border); border-radius: var(--radius);
      overflow: hidden;
    }
    table { width: 100%; border-collapse: collapse; text-align: left; font-size: 14px; }
    th { background: rgba(255, 255, 255, 0.03); padding: 14px 18px; color: var(--text-muted); font-weight: 600; border-bottom: 1px solid var(--card-border); }
    td { padding: 14px 18px; border-bottom: 1px solid rgba(255, 255, 255, 0.05); }
    tr:last-child td { border-bottom: none; }
    .perm-badge {
      display: inline-block; padding: 2px 8px; border-radius: 6px; background: rgba(255, 255, 255, 0.08);
      font-size: 12px; margin: 2px;
    }

    /* Modal */
    .modal-backdrop {
      position: fixed; inset: 0; background: rgba(0,0,0,0.7); backdrop-filter: blur(5px);
      display: none; align-items: center; justify-content: center; z-index: 500;
    }
    .modal {
      background: var(--card-bg); border: 1px solid var(--card-border); border-radius: var(--radius);
      padding: 28px; width: 100%; max-width: 480px; box-shadow: 0 20px 50px rgba(0,0,0,0.6);
    }
    .modal-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 20px; }
    .modal-title { font-size: 18px; font-weight: 600; }
    .modal-actions { display: flex; justify-content: flex-end; gap: 12px; margin-top: 24px; }

    .checkbox-list { display: flex; flex-direction: column; gap: 10px; max-height: 180px; overflow-y: auto; padding: 4px; }
    .checkbox-item { display: flex; align-items: center; gap: 10px; font-size: 14px; }
    .checkbox-item input[type="checkbox"] { width: 16px; height: 16px; accent-color: var(--accent); }
  </style>
</head>
<body>

  <!-- Login Modal -->
  <div id="login-overlay">
    <div class="login-box">
      <h1>⚡ ZapFast Server</h1>
      <p>Acesse o painel do servidor Docker</p>
      <form id="login-form">
        <div class="form-group">
          <label>Usuário</label>
          <input type="text" id="login-username" required placeholder="admin" autofocus>
        </div>
        <div class="form-group">
          <label>Senha</label>
          <input type="password" id="login-password" required placeholder="••••••••">
        </div>
        <div id="login-error" style="color: var(--danger); font-size: 13px; margin-bottom: 14px; display: none;"></div>
        <button type="submit" class="btn btn-primary btn-block">Entrar</button>
      </form>
    </div>
  </div>

  <!-- Header -->
  <header>
    <div class="logo-area">
      <div class="logo-dot"></div>
      <span>ZapFast Server</span>
      <span style="font-size: 12px; background: rgba(37,211,102,0.15); color: var(--accent); padding: 2px 8px; border-radius: 6px;">Docker / LAN</span>
    </div>
    <div class="header-user">
      <span id="current-user-display">admin</span>
      <button class="btn btn-secondary" onclick="logout()" style="padding: 6px 12px; font-size: 12px;">Sair</button>
    </div>
  </header>

  <!-- Navigation -->
  <div class="nav-tabs">
    <button class="nav-tab active" onclick="switchTab('instances')">📱 Contas WhatsApp</button>
    <button class="nav-tab" onclick="switchTab('users')">👥 Usuários & Permissões</button>
    <button class="nav-tab" onclick="switchTab('network')">🌐 Rede & Conexão</button>
  </div>

  <main>
    <!-- TAB: INSTANCES -->
    <div id="tab-instances">
      <div class="section-header">
        <div class="section-title">
          <h2>Instâncias do WhatsApp</h2>
          <p>Conecte números independentes com QR Code e anti-delete centralizado.</p>
        </div>
        <button class="btn btn-primary" onclick="openNewInstanceModal()">+ Conectar Nova Conta</button>
      </div>
      <div class="grid" id="instances-grid">
        <!-- Rendered by JS -->
      </div>
    </div>

    <!-- TAB: USERS -->
    <div id="tab-users" style="display: none;">
      <div class="section-header">
        <div class="section-title">
          <h2>Usuários do Servidor</h2>
          <p>Crie logins e defina quais números de WhatsApp cada pessoa pode acessar no app desktop.</p>
        </div>
        <button class="btn btn-primary" onclick="openNewUserModal()">+ Criar Novo Usuário</button>
      </div>
      <div class="table-container">
        <table>
          <thead>
            <tr>
              <th>Usuário</th>
              <th>Perfil</th>
              <th>WhatsApp(s) Autorizados</th>
              <th style="text-align: right;">Ações</th>
            </tr>
          </thead>
          <tbody id="users-table-body">
            <!-- Rendered by JS -->
          </tbody>
        </table>
      </div>
    </div>

    <!-- TAB: NETWORK -->
    <div id="tab-network" style="display: none;">
      <div class="section-header">
        <div class="section-title">
          <h2>Configurações de Rede & Descoberta</h2>
          <p>Dados para conexão a partir do aplicativo ZapFast Desktop.</p>
        </div>
      </div>
      <div class="card" style="max-width: 600px;">
        <div class="card-title">📡 Auto-Discovery na Rede Local</div>
        <p style="font-size: 13px; color: var(--text-muted);">
          O servidor emite beacons UDP na porta <strong>47120</strong>. No aplicativo ZapFast Desktop, basta clicar no botão <strong>"🔍 Buscar na Rede Local"</strong> para preencher o endereço automaticamente!
        </p>
        <hr style="border: none; border-top: 1px solid var(--card-border); margin: 8px 0;">
        <div class="form-group">
          <label>Endereço do Servidor para Conexão Manual</label>
          <input type="text" id="server-address-display" readonly>
        </div>
        <p style="font-size: 12px; color: var(--accent);">
          ✔ Anti-delete ativo permanentemente no banco central SQLCipher.
        </p>
      </div>
    </div>
  </main>

  <!-- Modal: Nova Instância -->
  <div class="modal-backdrop" id="modal-instance">
    <div class="modal">
      <div class="modal-header">
        <div class="modal-title">Conectar Conta de WhatsApp</div>
        <button class="btn btn-secondary" onclick="closeModal('modal-instance')" style="padding: 4px 8px;">✕</button>
      </div>
      <form id="new-instance-form">
        <div class="form-group">
          <label>Identificador da Conta (Ex: Vendas, Suporte, 01)</label>
          <input type="text" id="inst-id" required placeholder="suporte_01">
        </div>
        <div class="form-group">
          <label>Nome de Exibição</label>
          <input type="text" id="inst-name" required placeholder="Atendimento Suporte">
        </div>
        <div class="modal-actions">
          <button type="button" class="btn btn-secondary" onclick="closeModal('modal-instance')">Cancelar</button>
          <button type="submit" class="btn btn-primary">Iniciar Conexão</button>
        </div>
      </form>
    </div>
  </div>

  <!-- Modal: Novo Usuário -->
  <div class="modal-backdrop" id="modal-user">
    <div class="modal">
      <div class="modal-header">
        <div class="modal-title">Criar Novo Usuário</div>
        <button class="btn btn-secondary" onclick="closeModal('modal-user')" style="padding: 4px 8px;">✕</button>
      </div>
      <form id="new-user-form">
        <div class="form-group">
          <label>Nome de Usuário (Login)</label>
          <input type="text" id="user-uname" required placeholder="atendente_joao">
        </div>
        <div class="form-group">
          <label>Senha de Acesso</label>
          <input type="password" id="user-pwd" required placeholder="••••••••">
        </div>
        <div class="form-group">
          <label>Tipo de Perfil</label>
          <select id="user-role">
            <option value="operator">Operador (Acesso apenas aos WhatsApps permitidos)</option>
            <option value="admin">Administrador (Acesso total)</option>
          </select>
        </div>
        <div class="modal-actions">
          <button type="button" class="btn btn-secondary" onclick="closeModal('modal-user')">Cancelar</button>
          <button type="submit" class="btn btn-primary">Salvar Usuário</button>
        </div>
      </form>
    </div>
  </div>

  <!-- Modal: Editar Permissões -->
  <div class="modal-backdrop" id="modal-perms">
    <div class="modal">
      <div class="modal-header">
        <div class="modal-title">Permissões de WhatsApp</div>
        <button class="btn btn-secondary" onclick="closeModal('modal-perms')" style="padding: 4px 8px;">✕</button>
      </div>
      <div style="font-size: 13px; color: var(--text-muted); margin-bottom: 14px;">
        Selecione as contas de WhatsApp que o usuário <strong id="perm-user-name" style="color: var(--text);"></strong> poderá visualizar e atender:
      </div>
      <input type="hidden" id="perm-user-id">
      <div class="checkbox-list" id="perm-instance-list">
        <!-- Rendered by JS -->
      </div>
      <div class="modal-actions">
        <button type="button" class="btn btn-secondary" onclick="closeModal('modal-perms')">Cancelar</button>
        <button type="button" class="btn btn-primary" onclick="savePermissions()">Salvar Permissões</button>
      </div>
    </div>
  </div>

  <!-- QR Code Generator Library (minimal embedded canvas renderer) -->
  <script src="https://cdn.jsdelivr.net/npm/qrcode@1.5.3/build/qrcode.min.js"></script>
  <script>
    let token = localStorage.getItem('zapfast_token') || '';
    let currentUser = null;
    let allInstances = [];
    let allUsers = [];

    document.getElementById('server-address-display').value = window.location.host;

    if (token) {
      checkAuth();
    }

    document.getElementById('login-form').addEventListener('submit', async (e) => {
      e.preventDefault();
      const u = document.getElementById('login-username').value;
      const p = document.getElementById('login-password').value;
      const err = document.getElementById('login-error');
      err.style.display = 'none';

      try {
        const res = await fetch('/api/login', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ username: u, password: p })
        });
        const data = await res.json();
        if (data.token) {
          token = data.token;
          localStorage.setItem('zapfast_token', token);
          currentUser = data.user;
          document.getElementById('login-overlay').style.display = 'none';
          document.getElementById('current-user-display').innerText = currentUser.username;
          loadData();
        } else {
          err.innerText = data.error || 'Credenciais inválidas';
          err.style.display = 'block';
        }
      } catch (e) {
        err.innerText = 'Falha ao conectar com o servidor';
        err.style.display = 'block';
      }
    });

    async function checkAuth() {
      try {
        const res = await fetch('/api/me', { headers: { 'Authorization': 'Bearer ' + token } });
        const data = await res.json();
        if (data.user) {
          currentUser = data.user;
          document.getElementById('login-overlay').style.display = 'none';
          document.getElementById('current-user-display').innerText = currentUser.username;
          loadData();
          setInterval(loadData, 3000); // Polling live status
        } else {
          logout();
        }
      } catch (e) {
        logout();
      }
    }

    function logout() {
      token = '';
      localStorage.removeItem('zapfast_token');
      document.getElementById('login-overlay').style.display = 'flex';
    }

    function switchTab(tab) {
      document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
      event.target.classList.add('active');
      document.getElementById('tab-instances').style.display = tab === 'instances' ? 'block' : 'none';
      document.getElementById('tab-users').style.display = tab === 'users' ? 'block' : 'none';
      document.getElementById('tab-network').style.display = tab === 'network' ? 'block' : 'none';
    }

    async function loadData() {
      if (!token) return;
      try {
        const [instRes, userRes] = await Promise.all([
          fetch('/api/instances', { headers: { 'Authorization': 'Bearer ' + token } }),
          fetch('/api/users', { headers: { 'Authorization': 'Bearer ' + token } })
        ]);
        allInstances = await instRes.json();
        allUsers = await userRes.json();
        renderInstances();
        renderUsers();
      } catch (e) {
        console.error('Error loading data', e);
      }
    }

    function renderInstances() {
      const grid = document.getElementById('instances-grid');
      grid.innerHTML = '';
      if (!allInstances || allInstances.length === 0) {
        grid.innerHTML = '<div style="grid-column: 1/-1; text-align: center; color: var(--text-muted); padding: 40px;">Nenhuma conta de WhatsApp cadastrada. Clique em "+ Conectar Nova Conta" acima.</div>';
        return;
      }

      allInstances.forEach(inst => {
        const card = document.createElement('div');
        card.className = 'card';
        const isConnected = inst.connected || inst.status === 'connected';
        const badgeClass = isConnected ? 'status-connected' : (inst.qr ? 'status-unlinked' : 'status-error');
        const badgeText = isConnected ? '✔ Conectado' : (inst.qr ? 'Aguardando QR Code' : inst.status);

        let qrHtml = '';
        if (inst.qr) {
          qrHtml = `
            <div class="qr-container">
              <canvas id="qr-${inst.id}"></canvas>
              <div class="qr-instruction">Abra o WhatsApp no celular > Aparelhos conectados > Conectar um aparelho</div>
            </div>
          `;
        }

        card.innerHTML = `
          <div class="card-header">
            <div>
              <div class="card-title">${inst.name}</div>
              <div style="font-size: 12px; color: var(--text-muted);">${inst.phone || 'ID: ' + inst.id}</div>
            </div>
            <span class="status-badge ${badgeClass}">${badgeText}</span>
          </div>
          ${qrHtml}
          <div style="display: flex; gap: 8px; justify-content: flex-end; margin-top: auto;">
            <button class="btn btn-danger" onclick="deleteInstance('${inst.id}')" style="padding: 6px 12px; font-size: 12px;">Remover</button>
          </div>
        `;
        grid.appendChild(card);

        if (inst.qr && window.QRCode) {
          setTimeout(() => {
            const canvas = document.getElementById(`qr-${inst.id}`);
            if (canvas) {
              QRCode.toCanvas(canvas, inst.qr, { width: 180, margin: 1 });
            }
          }, 50);
        }
      });
    }

    function renderUsers() {
      const tbody = document.getElementById('users-table-body');
      tbody.innerHTML = '';
      if (!allUsers) return;

      allUsers.forEach(u => {
        const tr = document.createElement('tr');
        const perms = u.permissions || [];
        let permsText = '';
        if (u.role === 'admin') {
          permsText = '<span class="perm-badge" style="background: rgba(37,211,102,0.15); color: var(--accent);">Todas as Contas (Admin)</span>';
        } else if (perms.length === 0) {
          permsText = '<span style="color: var(--text-muted); font-size: 12px;">Nenhuma conta atribuída</span>';
        } else {
          permsText = perms.map(p => {
            const inst = allInstances.find(i => i.id === p);
            return `<span class="perm-badge">${inst ? inst.name : p}</span>`;
          }).join('');
        }

        tr.innerHTML = `
          <td><strong>${u.username}</strong></td>
          <td><span style="font-size: 12px; text-transform: uppercase; color: var(--text-muted);">${u.role}</span></td>
          <td>${permsText}</td>
          <td style="text-align: right;">
            ${u.role !== 'admin' ? `<button class="btn btn-secondary" onclick="openPermsModal('${u.id}', '${u.username}')" style="padding: 4px 10px; font-size: 12px; margin-right: 6px;">Permissões</button>` : ''}
            ${u.username !== 'admin' ? `<button class="btn btn-danger" onclick="deleteUser('${u.id}')" style="padding: 4px 10px; font-size: 12px;">Excluir</button>` : ''}
          </td>
        `;
        tbody.appendChild(tr);
      });
    }

    function openNewInstanceModal() { document.getElementById('modal-instance').style.display = 'flex'; }
    function openNewUserModal() { document.getElementById('modal-user').style.display = 'flex'; }
    function closeModal(id) { document.getElementById(id).style.display = 'none'; }

    document.getElementById('new-instance-form').addEventListener('submit', async (e) => {
      e.preventDefault();
      const id = document.getElementById('inst-id').value;
      const name = document.getElementById('inst-name').value;
      await fetch('/api/instances', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
        body: JSON.stringify({ id, name })
      });
      closeModal('modal-instance');
      loadData();
    });

    document.getElementById('new-user-form').addEventListener('submit', async (e) => {
      e.preventDefault();
      const username = document.getElementById('user-uname').value;
      const password = document.getElementById('user-pwd').value;
      const role = document.getElementById('user-role').value;
      await fetch('/api/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
        body: JSON.stringify({ username, password, role })
      });
      closeModal('modal-user');
      loadData();
    });

    async function deleteInstance(id) {
      if (!confirm('Deseja realmente remover esta conta de WhatsApp?')) return;
      await fetch(`/api/instances/${id}`, {
        method: 'DELETE',
        headers: { 'Authorization': 'Bearer ' + token }
      });
      loadData();
    }

    async function deleteUser(id) {
      if (!confirm('Deseja excluir este usuário?')) return;
      await fetch(`/api/users/${id}`, {
        method: 'DELETE',
        headers: { 'Authorization': 'Bearer ' + token }
      });
      loadData();
    }

    function openPermsModal(userId, username) {
      document.getElementById('perm-user-id').value = userId;
      document.getElementById('perm-user-name').innerText = username;
      const user = allUsers.find(u => u.id === userId);
      const userPerms = (user && user.permissions) || [];
      const list = document.getElementById('perm-instance-list');
      list.innerHTML = '';

      if (allInstances.length === 0) {
        list.innerHTML = '<div style="color: var(--text-muted); font-size: 13px;">Cadastre instâncias de WhatsApp primeiro.</div>';
      } else {
        allInstances.forEach(inst => {
          const item = document.createElement('label');
          item.className = 'checkbox-item';
          const checked = userPerms.includes(inst.id) ? 'checked' : '';
          item.innerHTML = `
            <input type="checkbox" value="${inst.id}" ${checked}>
            <span>${inst.name} (${inst.phone || inst.id})</span>
          `;
          list.appendChild(item);
        });
      }

      document.getElementById('modal-perms').style.display = 'flex';
    }

    async function savePermissions() {
      const userId = document.getElementById('perm-user-id').value;
      const checked = Array.from(document.querySelectorAll('#perm-instance-list input[type="checkbox"]:checked'))
        .map(cb => cb.value);

      await fetch(`/api/users/${userId}/permissions`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
        body: JSON.stringify({ account_ids: checked })
      });
      closeModal('modal-perms');
      loadData();
    }
  </script>
</body>
</html>
"#;
