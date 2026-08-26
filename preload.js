'use strict';
const { contextBridge, ipcRenderer } = require('electron');

// ── 状态页（file://）可用的桥接 API ──
contextBridge.exposeInMainWorld('dshDesktop', {
  onStatus: (cb) => ipcRenderer.on('backend-status', (_e, s) => cb(s)),
  onCountdown: (cb) => ipcRenderer.on('backend-countdown', (_e, s) => cb(s)),
  getStatus: () => ipcRenderer.send('app:get-status'),
  restart: () => ipcRenderer.send('backend:restart'),
  cancelRestart: () => ipcRenderer.send('backend:cancel-restart'),
  openBrowser: () => ipcRenderer.send('app:open-browser'),
  openLogs: () => ipcRenderer.send('app:open-logs'),
  quit: () => ipcRenderer.send('app:quit')
});

// ── 悬浮控制条（仅注入 dsh 后端页面）──
const isBackendPage = location.protocol === 'http:' || location.protocol === 'https:';
if (isBackendPage) {
  const CSS = `
#dsh-desktop-pill {
  position: fixed; top: 12px; left: 50%; transform: translateX(-50%);
  z-index: 2147483647; display: flex; align-items: center; gap: 8px;
  padding: 5px 10px; border-radius: 999px; cursor: default;
  background: rgba(15, 23, 42, 0.9); color: #e2e8f0;
  border: 1px solid rgba(148, 163, 184, 0.35);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
  font: 12px/1.4 "Segoe UI", "Microsoft YaHei", system-ui, sans-serif;
  user-select: none; -webkit-user-select: none; max-width: 60vw;
}
#dsh-desktop-pill.dshd-dragging { cursor: move; opacity: 0.85; }
#dsh-desktop-pill .dshd-dot {
  width: 8px; height: 8px; border-radius: 50%; flex: none;
  background: #94a3b8; transition: background 0.2s;
}
#dsh-desktop-pill .dshd-dot.dshd-ok { background: #22c55e; box-shadow: 0 0 6px rgba(34,197,94,.8); }
#dsh-desktop-pill .dshd-dot.dshd-warn { background: #eab308; box-shadow: 0 0 6px rgba(234,179,8,.8); }
#dsh-desktop-pill .dshd-dot.dshd-err { background: #ef4444; box-shadow: 0 0 6px rgba(239,68,68,.8); }
#dsh-desktop-pill .dshd-label { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
#dsh-desktop-pill .dshd-actions { display: flex; gap: 4px; flex: none; }
#dsh-desktop-pill button {
  background: rgba(255, 255, 255, 0.08); color: #e2e8f0;
  border: 1px solid rgba(148, 163, 184, 0.4); border-radius: 6px;
  font: 11px/1.2 "Segoe UI", "Microsoft YaHei", system-ui, sans-serif;
  padding: 2px 8px; cursor: pointer; flex: none;
}
#dsh-desktop-pill button:hover { background: rgba(255, 255, 255, 0.18); }
#dsh-desktop-pill .dshd-cancel { border-color: rgba(239, 68, 68, 0.6); color: #fca5a5; }
#dsh-desktop-pill.dshd-collapsed {
  padding: 5px 7px; cursor: pointer; background: rgba(15, 23, 42, 0.75);
}
#dsh-desktop-pill.dshd-collapsed .dshd-label,
#dsh-desktop-pill.dshd-collapsed .dshd-actions { display: none; }
`;
  const style = document.createElement('style');
  style.id = 'dsh-desktop-pill-style';
  style.textContent = CSS;

  function whenBody(cb, tries) {
    if (document.body) return cb();
    if ((tries || 0) > 300) return;
    setTimeout(() => whenBody(cb, (tries || 0) + 1), 100);
  }

  whenBody(() => {
    document.head.appendChild(style);

    const pill = document.createElement('div');
    pill.id = 'dsh-desktop-pill';
    pill.innerHTML =
      '<span class="dshd-dot"></span>' +
      '<span class="dshd-label">连接中…</span>' +
      '<span class="dshd-actions">' +
      '<button data-act="restart" title="重启 dsh 后端 (Ctrl+Shift+R)">重启</button>' +
      '<button data-act="cancel" class="dshd-cancel" title="取消自动重启" style="display:none">取消</button>' +
      '<button data-act="reload" title="刷新前端页面">⟳</button>' +
      '<button data-act="browser" title="在系统浏览器中打开">↗</button>' +
      '<button data-act="collapse" title="折叠">–</button>' +
      '</span>';
    document.body.appendChild(pill);

    const dot = pill.querySelector('.dshd-dot');
    const label = pill.querySelector('.dshd-label');
    const cancelBtn = pill.querySelector('.dshd-cancel');

    // ── 状态渲染 ──
    function renderStatus(s) {
      cancelBtn.style.display = 'none';
      switch (s.state) {
        case 'running':
        case 'external':
          dot.className = 'dshd-dot dshd-ok';
          label.textContent =
            s.state === 'external'
              ? `后端运行中（外部实例 · 端口 ${s.port}）`
              : `后端运行中 · 端口 ${s.port}`;
          pill.title = `${s.url || ''}  ·  pid ${s.pid || '?'}`;
          break;
        case 'starting':
          dot.className = 'dshd-dot dshd-warn';
          if (s.install) {
            label.textContent = s.install.phase === 'finishing'
              ? '下载完成，正在启动 dsh…'
              : `正在自动安装 dsh…（已获取 ${s.install.fetched || 0} 个包）`;
            pill.title = s.install.detail || '首次启动自动下载 dsh，需联网';
          } else {
            label.textContent = '正在启动后端…';
          }
          break;
        case 'restarting':
          dot.className = 'dshd-dot dshd-warn';
          label.textContent = '正在重启后端…';
          break;
        case 'error':
          dot.className = 'dshd-dot dshd-err';
          label.textContent = s.nextRetrySec
            ? `后端异常 · ${s.nextRetrySec}s 后重试`
            : '后端异常，等待重试';
          pill.title = s.error || 'dsh 后端异常';
          break;
        case 'stopped':
          dot.className = 'dshd-dot';
          label.textContent = '后端已停止';
          break;
        default:
          dot.className = 'dshd-dot dshd-warn';
          label.textContent = '后端未就绪';
      }
    }

    function renderCountdown(c) {
      dot.className = 'dshd-dot dshd-warn';
      label.textContent = `插件已变更 · ${c.seconds}s 后自动重启`;
      cancelBtn.style.display = '';
      pill.title = '插件已安装/移除，dsh 需要重启才能加载';
    }

    ipcRenderer.on('backend-status', (_e, s) => renderStatus(s));
    ipcRenderer.on('backend-countdown', (_e, c) => renderCountdown(c));
    ipcRenderer.send('app:get-status');

    // ── 按钮 ──
    pill.addEventListener('click', (e) => {
      const btn = e.target.closest('button');
      if (!btn) {
        if (pill.classList.contains('dshd-collapsed')) pill.classList.remove('dshd-collapsed');
        return;
      }
      const act = btn.dataset.act;
      if (act === 'restart') ipcRenderer.send('backend:restart');
      else if (act === 'cancel') ipcRenderer.send('backend:cancel-restart');
      else if (act === 'reload') ipcRenderer.send('app:reload');
      else if (act === 'browser') ipcRenderer.send('app:open-browser');
      else if (act === 'collapse') pill.classList.add('dshd-collapsed');
    });

    // ── 拖动 ──
    let dragging = false;
    let offX = 0;
    let offY = 0;
    pill.addEventListener('mousedown', (e) => {
      if (e.target.closest('button')) return;
      dragging = true;
      const r = pill.getBoundingClientRect();
      offX = e.clientX - r.left;
      offY = e.clientY - r.top;
      pill.classList.add('dshd-dragging');
      e.preventDefault();
    });
    window.addEventListener('mousemove', (e) => {
      if (!dragging) return;
      const x = Math.max(4, Math.min(window.innerWidth - pill.offsetWidth - 4, e.clientX - offX));
      const y = Math.max(4, Math.min(window.innerHeight - pill.offsetHeight - 4, e.clientY - offY));
      pill.style.left = x + 'px';
      pill.style.top = y + 'px';
      pill.style.transform = 'none';
    });
    window.addEventListener('mouseup', () => {
      dragging = false;
      pill.classList.remove('dshd-dragging');
    });
  });
}
