/* ═══ Inkling 原型交互逻辑 ═══ */
(() => {
  // ── 模拟数据 ──────────────────────────────────
  const clips = [
    { id: 1, type: 'text',  text: '把鼠标移到屏幕顶部中央试试 —— Inkling 的核心交互', date: todayStr(), time: '14:32:05', pinned: true },
    { id: 2, type: 'link',  text: 'https://tauri.app/zh-cn/v2/guides/', date: todayStr(), time: '14:10:47', pinned: false },
    { id: 3, type: 'code',  text: 'fn top_center(size: &PhysicalSize<u32>, origin: &PhysicalPosition<i32>)', date: todayStr(), time: '13:58:12', pinned: false },
    { id: 4, type: 'text',  text: '念头捕手的产品哲学：1 秒原则，零上下文切换', date: todayStr(), time: '11:24:36', pinned: false },
    { id: 5, type: 'image', text: '[图片] 设计稿-顶部面板-v3.png (1920×480)', date: yesterdayStr(), time: '18:02', pinned: false },
    { id: 6, type: 'text',  text: '#idea 桌面宠物 + 速记结合的玩法', date: yesterdayStr(), time: '09:15', pinned: false },
    // 历史种子：供侧边栏当月热力图与日期详情查询演示
    { id: 7, type: 'text',  text: 'GSAP timeline：滑入 200ms / 滑出 150ms，ease back.out(1.6)', date: daysAgoStr(20), time: '16:08', pinned: false },
    { id: 8, type: 'link',  text: 'https://docs.rs/tauri/2.0/window.html', date: daysAgoStr(12), time: '10:26', pinned: false },
    { id: 9, type: 'code',  text: 'select * from todos where date <= ? and done = 0;', date: daysAgoStr(5), time: '20:15', pinned: false },
  ];
  // 数据模型（待办 & 子任务一致）：
  //   完成时间 = date(完成日期) + dueTime(完成时刻)，两者必填
  //   remind = { date, time } | null 选填；未设置时默认「完成前30分钟/前5分钟/完成时」各提醒一次，设置后仅该时刻提醒
  //   tags ≤3 个、每个 ≤10 字；remark 备注 ≤200 字；children ≤5
  let todos = [
    { id: 1, text: '给产品文档补充截图', done: false, priority: 'high', date: todayStr(), dueTime: minutesFromNow(180), remind: { date: todayStr(), time: minutesFromNow(120) }, repeat: null, tags: ['产品'], remark: '需要覆盖首页、编辑器、归档流程三张截图，统一使用 2x 分辨率导出', children: [] },
    { id: 2, text: '评审 Inkling 架构文档 v2.0', done: false, priority: 'medium', date: todayStr(), dueTime: minutesFromNow(300), remind: { date: todayStr(), time: minutesFromNow(240) }, repeat: null, tags: ['架构', '评审'], remark: '混合模式演示备注：当备注长度超过一百个字的时候，卡片上仅显示一个图标徽章而不再展示文本行，鼠标悬浮在图标上即可查看完整的备注内容，点击图标则进入编辑弹窗进行修改。一百字以内的备注仍然以置灰文本行展示在任务内容下方，只占一行空间，超出部分使用省略号截断占位。', children: [
      { id: 11, text: '核对 SQLite 表结构', done: true, priority: 'medium', date: todayStr(), dueTime: minutesFromNow(60), remind: null, repeat: null, tags: [], remark: '', children: [] },
      { id: 12, text: '确认窗口属性矩阵', done: false, priority: 'low', date: todayStr(), dueTime: minutesFromNow(150), remind: null, repeat: null, tags: [], remark: '', children: [] },
    ] },
    { id: 3, text: '回复设计组毛玻璃反馈', done: false, priority: 'low', date: yesterdayStr(), dueTime: '18:00', remind: null, repeat: null, tags: [], remark: '', children: [] },  // 逾期示例
    { id: 4, text: '每日站会', done: false, priority: 'medium', date: todayStr(), dueTime: '09:30', remind: null, repeat: 'daily', tags: ['例会'], remark: '', children: [] },   // 完成时刻已过 → 当日逾期示例
    { id: 5, text: '昨天已完成的旧任务', done: true, priority: 'low', date: todayStr(), dueTime: minutesFromNow(-120), remind: null, repeat: null, tags: [], remark: '', children: [] },
    // 演示「父未逾期、子任务逾期」：父待办在明天，但子任务已逾期 → 整体归入当日列表并置顶
    { id: 6, text: '准备版本发布清单', done: false, priority: 'medium', date: tomorrowStr(), dueTime: '18:00', remind: null, repeat: null, tags: ['发布'], remark: '', children: [
      { id: 61, text: '确认更新日志文案', done: false, priority: 'high', date: yesterdayStr(), dueTime: '12:00', remind: null, repeat: null, tags: [], remark: '与法务确认开源条款表述', children: [] },
      { id: 62, text: '整理发布截图（已完成）', done: true, priority: 'low', date: yesterdayStr(), dueTime: '15:00', remind: null, repeat: null, tags: [], remark: '', children: [] },
    ] },
  ];
  function todayStr() { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`; }
  function nowTimeStr() { const d = new Date(); return `${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`; }   // 当前时间 HH:mm
  function yesterdayStr() { const d = new Date(Date.now() - 864e5); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`; }
  function tomorrowStr() { const d = new Date(Date.now() + 864e5); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`; }
  function minutesFromNow(mins) { const d = new Date(Date.now() + mins * 60000); return `${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`; }   // 当前时刻 ± 分钟 → HH:mm
  function daysAgoStr(n) { const d = new Date(Date.now() - n * 864e5); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`; }   // n 天前日期
  const PRIORITY = { high: { label: '高', color: 'high' }, medium: { label: '中', color: 'medium' }, low: { label: '低', color: 'low' } };
  const PRIORITY_ORDER = { high: 0, medium: 1, low: 2 };
  const MAX_CHILDREN = 5;
  let notes = [
    { id: 1, text: '**Inkling 1 秒原则**：从念头产生到文字落屏必须 < 1 秒，全程不切换当前应用。', tags: ['产品', '核心原则'], date: todayStr(), time: '今天 14:02', pinned: false },
    { id: 2, text: '桌面感应区方案：常驻透明窗口 > 鼠标轮询（零 CPU 开销）', tags: ['架构', '技术选型', '性能', '窗口系统'], date: todayStr(), time: '今天 11:20', pinned: true },
    { id: 3, text: '看到一个很棒的动效库 GSAP，物理弹性很适合面板滑入', tags: ['灵感'], date: yesterdayStr(), time: '昨天 17:45', pinned: false },
    { id: 4, text: '没有任何标签的笔记示例', tags: [], date: yesterdayStr(), time: '昨天 09:10', pinned: false },
    // 历史种子：供侧边栏当月热力图与日期详情查询演示
    { id: 5, text: '热力图配色：蓝阶四档 + 逾期红框，弱化网格线', tags: ['设计'], date: daysAgoStr(24), time: '10:12', pinned: false },
    { id: 6, text: '`SQLite` WAL 模式读写并发验证通过', tags: ['技术'], date: daysAgoStr(15), time: '16:40', pinned: false },
    { id: 7, text: '毛玻璃在 Windows 下的降级方案：Acrylic → 纯色', tags: ['架构', '兼容'], date: daysAgoStr(8), time: '09:31', pinned: false },
  ];

  // ── DOM 引用 ──────────────────────────────────
  const $ = (id) => document.getElementById(id);
  const panel = $('panel'), hotzone = $('hotzone'), editor = $('editor');
  const saveState = $('saveState'), tagPreview = $('tagPreview');

  // 标签约束常量
  const TAG_MAX_SHOW = 3;   // 笔记卡片最多展示 3 个标签
  const TAG_MAX_LEN = 5;    // 每个标签最大 5 字
  const escapeAttr = (s) => String(s).replace(/&/g,'&amp;').replace(/"/g,'&quot;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
  const escapeHtml = (s) => String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
  // 搜索命中高亮：转义后包裹首个命中片段（忽略大小写）
  function hiText(text, q) {
    const esc = escapeHtml(text);
    if (!q) return esc;
    const i = esc.toLowerCase().indexOf(q.toLowerCase());
    if (i < 0) return esc;
    return esc.slice(0, i) + '<mark>' + esc.slice(i, i + q.length) + '</mark>' + esc.slice(i + q.length);
  }
  // 轻量 Markdown 渲染（卡片展示用）：**粗体**、`代码`，换行转 <br>；先转义防注入
  function renderMdCard(text) {
    return String(text)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/\n/g, '<br>');
  }

  // ── 删除二次确认状态（笔记/粘贴板/待办统一：✕ → 确认浮层 → 删除） ──
  let noteConfirmId = null;   // 历史归档·笔记
  let clipConfirmId = null;   // 粘贴板条目（面板与归档共用一套数据）

  // ── Toast ─────────────────────────────────────
  let toastTimer;
  function toast(msg) {
    const t = $('toast');
    t.textContent = msg; t.classList.remove('hidden');
    gsap.fromTo(t, { y: 20, opacity: 0 }, { y: 0, opacity: 1, duration: .25 });
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => gsap.to(t, { opacity: 0, y: 10, duration: .25, onComplete: () => t.classList.add('hidden') }), 1800);
  }

  // ── 面板展开/收起（GSAP 物理动画） ────────────
  let panelVisible = false, hideTimer;
  // 弹窗守卫：任何编辑弹窗（待办/标签/剪贴板）打开期间，面板不因失焦收起（弹窗内容需回显到面板）
  let modalDepth = 0;
  function modalOpened() {
    modalDepth++;
    clearTimeout(hideTimer);              // 取消已排期的自动收起
  }
  function modalClosed() {
    modalDepth = Math.max(0, modalDepth - 1);
    // 全部弹窗关闭后，若鼠标不在面板内，重新按延迟策略排期收起
    if (modalDepth === 0 && panelVisible && !panel.matches(':hover')) {
      clearTimeout(hideTimer);
      hideTimer = setTimeout(hidePanel, 3000);
    }
  }
  function showPanel() {
    if (panelVisible) return;
    panelVisible = true;
    panel.classList.remove('hidden');
    gsap.fromTo(panel,
      { y: -30, opacity: 0, scale: .97 },
      { y: 0, opacity: 1, scale: 1, duration: .28, ease: 'back.out(1.6)' }); // 200ms 弹性滑入
    renderPanelTags();   // 呼出时刷新右下角标签区（草稿标签或"无标签"）
    setTimeout(() => editor.focus(), 100);
  }
  function hidePanel() {
    if (!panelVisible) return;
    if (modalDepth > 0) return;           // 有编辑弹窗打开时禁止收起（内容需回显）
    panelVisible = false;
    // 若正在编辑笔记但未保存（Esc/失焦放弃），退出编辑模式并恢复按钮文案
    if (typeof editingNoteId !== 'undefined' && editingNoteId !== null) {
      editingNoteId = null;
      $('btnArchive').textContent = '归档念头 ↵';
      editor.innerHTML = ''; renderPanelTags();
    }
    gsap.to(panel, {
      y: -24, opacity: 0, duration: .18, ease: 'power2.in',   // 150ms 收起
      onComplete: () => panel.classList.add('hidden')
    });
  }

  // hotzone 悬停 100ms 防抖展开
  let hoverTimer;
  hotzone.addEventListener('mouseenter', () => { hoverTimer = setTimeout(showPanel, 100); });
  hotzone.addEventListener('mouseleave', () => clearTimeout(hoverTimer));
  // 面板失焦 → 延迟 3s 自动收起（对应设置项）
  panel.addEventListener('mouseleave', () => {
    if (modalDepth > 0) return;           // 弹窗打开时：不因失焦收起
    hideTimer = setTimeout(hidePanel, 3000);
  });
  panel.addEventListener('mouseenter', () => clearTimeout(hideTimer));

  // 键盘：Esc 收起 / ⌃1-3 切态 / ⌃⇧Space 呼出
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      // 标签管理弹窗 / 编辑浮框打开时优先关闭，不触发面板收起
      if (!$('tagManagerOverlay').classList.contains('hidden')) { closeTagManager(); return; }
      if (!$('clipEditorOverlay').classList.contains('hidden')) { closeClipEditor(); return; }
      if (!$('todoEditorOverlay').classList.contains('hidden')) { closeTodoEditor(); return; }
      if (!$('prioMenu').classList.contains('hidden')) { hidePrioMenu(); return; }
      if (!$('repeatMenu').classList.contains('hidden')) { hideRepeatMenu(); return; }
      hidePanel(); closeAllWindows(); $('trayMenu').classList.add('hidden');
    }
    if (e.ctrlKey && e.shiftKey && e.code === 'Space') { e.preventDefault(); panelVisible ? hidePanel() : showPanel(); }
    if (e.ctrlKey && ['Digit1','Digit2','Digit3'].includes(e.code)) {
      switchMode(['note','clipboard','todo'][Number(e.code.slice(-1)) - 1]);
    }
    if (e.key === 'Enter' && !e.shiftKey && document.activeElement === editor) {
      e.preventDefault(); archiveNote();
    }
  });

  // ── 三态圆点切换 ──────────────────────────────
  function switchMode(mode) {
    document.querySelectorAll('.nav-dot').forEach(d => d.classList.toggle('active', d.dataset.mode === mode));
    ['note','clipboard','todo'].forEach(m => $('page-' + m).classList.toggle('hidden', m !== mode));
    if (mode === 'clipboard') renderClips();
    if (mode === 'todo') renderTodos();
  }
  document.querySelectorAll('.nav-dot').forEach(d => d.addEventListener('click', () => switchMode(d.dataset.mode)));

  // ── 🔴 笔记：即时渲染 + 自动保存 + 归档 ───────
  let saveTimer;
  editor.addEventListener('input', () => {
    saveState.textContent = '输入中…'; saveState.classList.add('saving');
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {   // 500ms 防抖自动暂存
      saveState.textContent = '已暂存 SQLite'; saveState.classList.remove('saving');
    }, 500);
    renderInlineMarkdown();
  });

  // 轻量即时渲染：**粗体** `代码`（标签已从文本剥离，不再解析 #标签）
  function renderInlineMarkdown() {
    const sel = window.getSelection();
    let html = editor.innerText
      .replace(/&/g,'&amp;').replace(/</g,'&lt;')
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/\n/g, '<br>');
    if (editor.innerHTML !== html) {
      editor.innerHTML = html;
      // 光标复位（原型简化处理：移到末尾）
      const range = document.createRange();
      range.selectNodeContents(editor); range.collapse(false);
      sel.removeAllRanges(); sel.addRange(range);
    }
  }

  // ── 标签区渲染（面板右下角 / 卡片左下角共用规则） ──
  // 面板右下角标签区：编辑已有笔记时显示该笔记标签，新建时显示草稿标签
  let draftTags = [];   // 新建笔记的草稿标签（未归档）

  function renderPanelTags() {
    const tags = editingNoteId !== null
      ? (notes.find(x => x.id === editingNoteId)?.tags ?? [])
      : draftTags;
    if (tags.length === 0) {
      tagPreview.innerHTML = '<span class="tag-empty" title="点击管理标签">无标签</span>';
      return;
    }
    const shown = tags.slice(0, TAG_MAX_SHOW);
    let html = shown.map(t => {
      const display = t.length > TAG_MAX_LEN ? t.slice(0, TAG_MAX_LEN) + '…' : t;
      return `<span class="tag-chip panel-tag" data-tag="${escapeAttr(t)}" title="#${escapeAttr(t)}"><span class="tag-name">${escapeAttr(display)}</span></span>`;
    }).join('');
    if (tags.length > TAG_MAX_SHOW) html += `<span class="tag-more" title="查看全部标签">+${tags.length - TAG_MAX_SHOW}</span>`;
    tagPreview.innerHTML = html;
  }
  // 点击面板标签区 → 打开标签管理弹窗（目标：当前编辑笔记 或 草稿）
  tagPreview.addEventListener('click', () => {
    openTagManager(editingNoteId !== null ? { type: 'note', id: editingNoteId } : { type: 'draft' });
  });

  // 编辑状态：非 null 表示正在编辑已有笔记（归档时更新而非新建）
  let editingNoteId = null;

  function archiveNote() {
    const text = editor.innerText.trim();
    if (!text) { toast('先写点什么吧'); return; }
    const size = new Blob([text]).size;
    const over = size > 1024 * 1024;

    if (editingNoteId !== null) {
      // 编辑模式：更新原笔记（时间刷新为最后修改时间，置顶与标签保留）
      const n = notes.find(x => x.id === editingNoteId);
      if (n) { n.text = text; n.time = '刚刚（已编辑）'; }
      editingNoteId = null;
      $('btnArchive').textContent = '归档念头 ↵';
      toast('修改已保存 ✔');
    } else {
      // 新建：携带草稿标签（标签与文本解耦，不从正文解析）
      notes.unshift({ id: Date.now(), text, tags: draftTags.slice(), date: todayStr(), time: '刚刚', pinned: false });
      draftTags = [];
      toast(over ? '内容超过 1MB，已自动落盘 /notes/*.md ✔' : '念头已归档 ✔');
    }
    editor.innerHTML = ''; renderPanelTags();
    renderArchive();   // 归档/编辑保存后同步归档列表与日期详情
    setTimeout(hidePanel, 250);
  }
  $('btnArchive').addEventListener('click', archiveNote);

  // ── 🟡 粘贴板：列表/搜索/双击置顶/收藏/编辑 ──
  function renderClips(filter = '') {
    const list = $('clipList');
    const TEXT_EDITABLE = ['text', 'link', 'code', 'richtext'];
    const items = clips.filter(c => c.text.toLowerCase().includes(filter.toLowerCase()))
      .sort((a,b) => (b.pinned - a.pinned));
    list.innerHTML = items.map(c => `
      <li class="clip-item ${c.pinned ? 'pinned' : ''}" data-id="${c.id}">
        ${clipConfirmId === c.id ? `<div class="card-confirm">
          <span class="card-confirm-text">确认删除该条目？</span>
          <button class="btn tiny danger" data-op="del-yes">删除</button>
          <button class="btn tiny ghost" data-op="del-no">取消</button>
        </div>` : ''}
        <!-- 右上：删除图标按钮（悬浮卡片时显示） -->
        <button class="card-close" data-op="del" title="删除该条目">${ICON_CLOSE}</button>
        <!-- 左上：时间 -->
        <div class="clip-head">
          <span class="clip-time">${c.pinned ? '📌 ' : ''}${c.time}</span>
          <span class="clip-type ${c.type}">${{text:'文本',link:'链接',code:'代码',image:'图片'}[c.type]}</span>
        </div>
        <div class="clip-text">${c.text}</div>
        <!-- 右下：粘贴 / 打开链接（仅 link 类型）/ 编辑 / 收藏 图标按钮（悬浮卡片时显示） -->
        <div class="clip-ops">
          <button class="icon-btn" data-op="paste" title="粘贴（写回剪贴板）">${ICON_PASTE}</button>
          ${c.type === 'link' ? `<a class="icon-btn clip-open" href="${c.text}" target="_blank" rel="noopener noreferrer" title="用默认浏览器打开该链接">${ICON_LINK}</a>` : ''}
          ${TEXT_EDITABLE.includes(c.type) ? `<button class="icon-btn" data-op="edit" title="编辑内容">${ICON_EDIT}</button>` : ''}
          <button class="icon-btn ${c.pinned ? 'active-pin' : ''}" data-op="pin" title="${c.pinned ? '取消收藏' : '收藏置顶'}">${ICON_PIN}</button>
        </div>
      </li>`).join('');
  }
  $('clipSearch').addEventListener('input', (e) => renderClips(e.target.value));
  $('clipList').addEventListener('dblclick', (e) => {
    const item = e.target.closest('.clip-item'); if (!item) return;
    const c = clips.find(x => x.id == item.dataset.id);
    c.pinned = true;   // 双击 = 粘贴并置顶
    toast('已粘贴，并置顶该条目 ✔');
    renderClips($('clipSearch').value);
  });
  $('clipList').addEventListener('click', (e) => {
    const btn = e.target.closest('[data-op]'); if (!btn) return;
    const id = e.target.closest('.clip-item').dataset.id;
    const c = clips.find(x => x.id == id);
    const op = btn.dataset.op;
    if (op === 'paste') toast('已写回剪贴板 ✔');
    if (op === 'pin') { c.pinned = !c.pinned; toast(c.pinned ? '已收藏 📌' : '已取消收藏'); }
    if (op === 'edit') { openClipEditor(c); return; }   // 打开独立编辑浮框（内部负责重渲染）
    if (op === 'del') { clipConfirmId = c.id; }         // 先确认再删除
    if (op === 'del-yes') {
      clips.splice(clips.indexOf(c), 1); clipConfirmId = null;
      toast('条目已删除'); renderArchive();
    }
    if (op === 'del-no') { clipConfirmId = null; }
    renderClips($('clipSearch').value);
  });

  // ── 粘贴板内容编辑浮框（纯编辑职责，与置顶解耦） ──
  let editingClipId = null;
  function openClipEditor(c) {
    editingClipId = c.id;
    document.querySelector('#clipEditorModal .clip-editor-title').textContent = '✏️ 编辑剪贴板内容';
    $('clipEditorTextarea').value = c.text;   // 回显原内容
    $('clipEditorOverlay').classList.remove('hidden');
    modalOpened();
    gsap.fromTo('#clipEditorModal', { scale: .94, opacity: 0 }, { scale: 1, opacity: 1, duration: .2, ease: 'power2.out' });
    setTimeout(() => { $('clipEditorTextarea').focus(); $('clipEditorTextarea').setSelectionRange(c.text.length, c.text.length); }, 80);
  }
  function closeClipEditor() {
    editingClipId = null;
    gsap.to('#clipEditorModal', { scale: .96, opacity: 0, duration: .15, ease: 'power2.in',
      onComplete: () => { $('clipEditorOverlay').classList.add('hidden'); modalClosed(); } });
  }
  function saveClipEdit() {
    const c = clips.find(x => x.id === editingClipId);
    const next = $('clipEditorTextarea').value.trim();
    if (c && next) {
      c.text = next;                                     // 替换原内容
      c.time = new Date().toTimeString().slice(0, 8) + '（已编辑）';  // 时间更新为最后修改时间
      toast('已保存修改 ✔');
    } else if (c && !next) {
      toast('内容为空，未保存');
    }
    closeClipEditor();
    renderClips($('clipSearch').value);   // 面板列表刷新
    renderArchive();                       // 归档页同步刷新（若打开）
  }
  $('clipEditorSave').addEventListener('click', saveClipEdit);
  $('clipEditorCancel').addEventListener('click', closeClipEditor);
  $('clipEditorClose').addEventListener('click', closeClipEditor);
  $('clipEditorOverlay').addEventListener('click', (e) => { if (e.target === e.currentTarget) closeClipEditor(); });
  // ⌃/⌘+Enter 快捷保存
  $('clipEditorTextarea').addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); saveClipEdit(); }
  });

  // ── 🟢 待办：排序 / 渲染 / 交互引擎 ────────────
  // 排序规则：按完成时间升序 → 优先级高在前；已完成的沉底（仍按完成时间、优先级）
  const dueKey = (t) => `${t.date} ${t.dueTime || '23:59'}`;
  function todoLevel(t) { return t.done ? 1 : 0; }
  function sortTodos(list) {
    return [...list].sort((a, b) =>
      (todoLevel(a) - todoLevel(b)) ||
      dueKey(a).localeCompare(dueKey(b)) ||
      (PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority]));
  }
  // 逾期：未完成且完成时刻已过。查看过去日期 → 该日未完成均视为逾期；查看今天 → 完成时刻≤当前；未来日期无逾期
  function isOverdue(t, viewDate) {
    if (t.done) return false;
    if (t.date < viewDate) return true;
    if (t.date > viewDate) return false;
    if (viewDate < todayStr()) return true;
    if (viewDate > todayStr()) return false;
    return t.dueTime <= nowTimeStr();
  }
  function allChildrenDone(t) { return t.children.length > 0 && t.children.every(c => c.done); }
  function findTodo(id) {
    for (const t of todos) {
      if (t.id === id) return t;
      const c = t.children.find(x => x.id === id);
      if (c) return c;
    }
    return null;
  }

  let todoDeleteConfirmId = null;   // 删除二次确认中的待办 id

  // 备注展示样式（设置页配置）：auto=混合（≤100字文本行 / >100字图标徽章）、icon、line
  function remarkDisplayStyle() {
    const sel = document.getElementById('settingRemarkStyle');
    return sel ? sel.value : 'auto';
  }

  // 单条待办 HTML（含子任务树；depth 供缩进）
  // opts: { showDate:搜索结果显示所属日期, q:搜索词(高亮), forceExpand:命中子任务时强制展开, hitId:命中的子任务 id }
  function todoItemHTML(t, viewDate, depth = 0, opts = {}) {
    const overdue = isOverdue(t, viewDate);
    const canEdit = !t.done;                                   // 已完成：禁改禁删
    const hasChildren = t.children.length > 0;
    const collapsed = !!t.collapsed;
    const expanded = hasChildren && (!collapsed || !!opts.forceExpand);   // 命中子任务时强制展开
    const hit = opts.hitId === t.id ? 'search-hit' : '';
    // 备注展示：图标徽章（徽章区，悬浮 tooltip）或置灰文本行（任务内容下方单行省略，悬浮全文）
    const style = remarkDisplayStyle();
    const remark = t.remark || '';
    const useRemarkIcon = !!remark && (style === 'icon' || (style === 'auto' && remark.length > 100));
    const useRemarkLine = !!remark && !useRemarkIcon;
    // 提醒徽章：已设置 → 日期+时间（同日仅显时间）；未设置 → 淡色占位（默认三次提醒）
    const remindText = t.remind
      ? (t.remind.date === t.date ? `⏰ ${t.remind.time}` : `⏰ ${Number(t.remind.date.slice(5, 7))}/${Number(t.remind.date.slice(8))} ${t.remind.time}`)
      : '⏰';
    const remindTitle = t.remind
      ? `提醒：${t.remind.date} ${t.remind.time}（点击修改）`
      : '默认提醒：完成前30分钟、前5分钟、完成时各一次（点击设置）';
    const tagsHtml = (t.tags && t.tags.length)
      ? `<div class="todo-tags">${t.tags.map(tg =>
          `<span class="tag-chip todo-tag" data-todoact="edit" title="#${escapeAttr(tg)}（点击编辑）"><span class="tag-name">${escapeAttr(tg)}</span></span>`).join('')}</div>`
      : '';
    // 完成时间徽章（紧跟标签之后，可点击修改）
    const dueText = t.date === todayStr()
      ? `📅 今天 ${t.dueTime}`
      : `📅 ${Number(t.date.slice(5, 7))}/${Number(t.date.slice(8))} ${t.dueTime}`;
    const dueChip = `<span class="due-badge ${overdue ? 'overdue' : ''}" data-todoact="due" title="完成时间：${escapeAttr(t.date + ' ' + t.dueTime)}（点击修改）">${dueText}</span>`;
    // 操作区：已完成项禁改（⏰/✏️ 隐藏），但父级仍可 ＋子任务（新建后系统自动恢复父级为未完成）
    const canAddChild = depth === 0 && t.children.length < MAX_CHILDREN;
    const opsHtml = (canEdit || canAddChild) ? `
          <div class="todo-ops">
            ${canEdit ? `<span class="remind-badge ${t.remind ? '' : 'no-remind'}" data-todoact="remind" title="${escapeAttr(remindTitle)}">${remindText}</span>` : ''}
            ${canAddChild ? `<button class="icon-btn" data-todoact="addchild" title="添加子任务（${t.children.length}/${MAX_CHILDREN}）${t.done ? ' · 新建后自动恢复为未完成' : ''}">＋</button>` : ''}
            ${canEdit ? `<button class="icon-btn" data-todoact="edit" title="编辑内容">${ICON_EDIT}</button>` : ''}
          </div>` : '';
    return `
      <li class="todo-item prio-${t.priority} ${t.done ? 'done' : ''} ${overdue ? 'overdue' : ''} depth-${depth} ${hasChildren ? 'has-children' : ''} ${collapsed ? 'collapsed' : ''} ${hit}" data-id="${t.id}">
        ${todoDeleteConfirmId === t.id ? `<div class="card-confirm">
          <span class="card-confirm-text">⚠️ 确认删除该${depth === 0 ? '待办事项' : '子任务'}？</span>
          <button class="btn tiny danger" data-todoact="del-yes">删除</button>
          <button class="btn tiny ghost" data-todoact="del-no">取消</button>
        </div>` : ''}
        <div class="todo-body">
        ${t.done ? '' : `<button class="card-close todo-del" data-todoact="del" title="删除待办">${ICON_CLOSE}</button>`}
        <div class="todo-head">
        <span class="tree-toggle ${hasChildren ? '' : 'leaf'} ${collapsed ? 'closed' : ''}"
              ${hasChildren ? 'data-todoact="collapse"' : ''} title="${collapsed ? '展开子任务' : '折叠子任务'}">▸</span>
        <span class="checkbox" data-todoact="toggle">${t.done ? '✓' : ''}</span>
        <div class="todo-main">
          <div class="todo-row">
            <span class="prio-badge ${t.priority}" data-todoact="prio" title="优先级：${PRIORITY[t.priority].label}（点击修改）">${PRIORITY[t.priority].label}</span>
            <span class="todo-text">${hiText(t.text, opts.q)}</span>
            <span class="todo-badges">
              ${opts.showDate ? `<span class="todo-date-chip" title="所属日期">${t.date}${t.date === todayStr() ? ' · 今天' : ''}</span>` : ''}
              ${overdue ? '<span class="overdue-flag" title="完成时间已过">逾期</span>' : ''}
              ${t.repeat ? `<span class="todo-meta repeat" title="重复提醒：${t.repeat === 'daily' ? '每天' : '每周'}（点击切换/结束）" data-todoact="repeat">🔁${t.repeat === 'daily' ? '每天' : '每周'}</span>` : ''}
              ${useRemarkIcon ? `<span class="remark-badge" data-todoact="edit" title="${escapeAttr(remark)}">📄</span>` : ''}
            </span>
          </div>
          ${useRemarkLine ? `<div class="todo-remark" data-todoact="edit" title="${escapeAttr(remark)}">${escapeHtml(remark)}</div>` : ''}
        </div>
        </div>
        ${(tagsHtml || opsHtml) ? `<div class="todo-foot"><div class="todo-foot-left">${tagsHtml}${dueChip}</div>${opsHtml}</div>` : ''}
        </div>
        ${hasChildren && expanded ? `<ul class="todo-children">${sortTodos(t.children).map(c => todoItemHTML(c, viewDate, depth + 1, opts)).join('')}</ul>` : ''}
      </li>`;
  }

  // 逾期分区排序：完成时间升序 → 优先级
  function sortOverdue(list) {
    return [...list].sort((a, b) =>
      dueKey(a).localeCompare(dueKey(b)) ||
      (PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority]));
  }

  // 共享渲染器：viewDate 为查看日；面板传当天，归档页随日期切换
  // 逾期规则：待办自身逾期，或其任一子任务逾期 → 整体（含全部子任务，含已完成）归入当日列表顶部逾期区
  // q 非空时进入搜索模式：跨全部日期模糊匹配（含子任务文本），结果显示所属日期
  function renderTodoList(el, viewDate, q = '') {
    if (q) {
      const needle = q.toLowerCase();
      const match = (t) => t.text.toLowerCase().includes(needle);
      const hits = todos.filter(t => match(t) || t.children.some(match))
        .sort((a, b) => b.date.localeCompare(a.date) ||
          PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority]);
      el.innerHTML = hits.map(t => {
        const parentHit = match(t);
        const childOpts = parentHit
          ? { showDate: true, q, hitId: t.children.find(match)?.id }   // 命中待办：显示整棵子任务树
          : { showDate: true, q, forceExpand: true, hitId: t.children.find(match)?.id };  // 仅命中子任务：展开并高亮
        return todoItemHTML(t, viewDate, 0, childOpts);
      }).join('') || `<div class="todo-empty">未找到匹配「${escapeHtml(q)}」的待办事项</div>`;
      return;
    }
    const pulled = todos.filter(t =>
      isOverdue(t, viewDate) || t.children.some(c => isOverdue(c, viewDate)));
    const pulledIds = new Set(pulled.map(t => t.id));
    const normal = todos.filter(t => t.date === viewDate && !pulledIds.has(t.id));
    const pulledHtml = pulled.length
      ? `<li class="todo-section">⚠️ 逾期事项 · 按完成时间与优先级置顶（${pulled.length} 项）</li>` +
        sortOverdue(pulled).map(t => todoItemHTML(t, viewDate)).join('')
      : '';
    const normalHtml = normal.length ? sortTodos(normal).map(t => todoItemHTML(t, viewDate)).join('') : '';
    el.innerHTML = (pulledHtml + normalHtml) || `<div class="todo-empty">该日暂无待办事项</div>`;
  }

  // ── 重复提醒下拉选择 ──────────────────────────
  let repeatMenuTodoId = null;
  function showRepeatMenu(t, anchorEl) {
    repeatMenuTodoId = t.id;
    const menu = $('repeatMenu');
    // 高亮当前项
    menu.querySelectorAll('.repeat-opt').forEach(o =>
      o.classList.toggle('active', o.dataset.repeat === (t.repeat || '')));
    menu.classList.remove('hidden');
    const r = anchorEl.getBoundingClientRect();
    menu.style.top = (r.bottom + 6) + 'px';
    menu.style.left = Math.min(r.left, innerWidth - 180) + 'px';
    gsap.fromTo(menu, { y: -6, opacity: 0 }, { y: 0, opacity: 1, duration: .18, ease: 'power2.out' });
  }
  function hideRepeatMenu() { $('repeatMenu').classList.add('hidden'); repeatMenuTodoId = null; }
  $('repeatMenu').addEventListener('click', (e) => {
    const opt = e.target.closest('.repeat-opt'); if (!opt) return;
    const t = findTodo(repeatMenuTodoId); if (!t) { hideRepeatMenu(); return; }
    t.repeat = opt.dataset.repeat || null;
    toast(t.repeat ? '重复提醒：' + (t.repeat === 'daily' ? '每天' : '每周') : '已设为不重复');
    hideRepeatMenu();
    renderTodos();
  });
  document.addEventListener('click', (e) => {
    if (!e.target.closest('#repeatMenu') && !e.target.closest('[data-todoact="repeat"]')) hideRepeatMenu();
    if (!e.target.closest('#prioMenu') && !e.target.closest('[data-todoact="prio"]')) hidePrioMenu();
  });

  // ── 优先级阶梯下拉（点击待办的优先级徽章触发） ──
  // 当前优先级由 Pin 卡片本身充当，不重复出现在列表中：
  //   低 → 菜单悬浮卡片上方（从上到下 高→中）；高 → 悬浮下方（中→低）；中 → 高在上、低在下
  let prioMenuTodoId = null, prioMenuCard = null;
  function showPrioMenu(t, badgeEl) {
    const cardEl = badgeEl.closest('.todo-item');
    hidePrioMenu();
    prioMenuTodoId = t.id; prioMenuCard = cardEl;
    const menu = $('prioMenu');
    const opt = (p) => `<div class="prio-opt ${p}" data-prio="${p}"><span class="prio-dot"></span>${PRIORITY[p].label}优先级</div>`;
    const up = [], down = [];
    if (t.priority === 'low') { up.push('high', 'medium'); }
    else if (t.priority === 'medium') { up.push('high'); down.push('low'); }
    else { down.push('medium', 'low'); }
    menu.innerHTML =
      (up.length ? `<div class="prio-group up">${up.map(opt).join('')}</div>` : '') +
      (down.length ? `<div class="prio-group down">${down.map(opt).join('')}</div>` : '');
    menu.classList.remove('hidden');
    cardEl.classList.add('prio-ladder');   // 卡片高亮描边 = 当前优先级阶梯
    // 定位：以本条目内容行（todo-body，不含子任务树）为锚点，上下浮层紧贴卡片，与徽章左对齐（防出屏）
    const rowRect = (badgeEl.closest('.todo-body') || cardEl).getBoundingClientRect();
    const badgeLeft = badgeEl.getBoundingClientRect().left;
    const left = Math.max(8, Math.min(badgeLeft, innerWidth - 112));
    const upEl = menu.querySelector('.prio-group.up');
    const downEl = menu.querySelector('.prio-group.down');
    if (upEl) {
      upEl.style.left = left + 'px';
      let top = rowRect.top - 6 - upEl.offsetHeight;
      if (top < 8) top = rowRect.bottom + 6;   // 顶部放不下则翻转到下方
      upEl.style.top = top + 'px';
      gsap.fromTo(upEl, { y: 6, opacity: 0 }, { y: 0, opacity: 1, duration: .16, ease: 'power2.out' });
    }
    if (downEl) {
      downEl.style.left = left + 'px';
      let top = rowRect.bottom + 6;
      if (top + downEl.offsetHeight > innerHeight - 8) top = Math.max(8, rowRect.top - 6 - downEl.offsetHeight);
      downEl.style.top = top + 'px';
      gsap.fromTo(downEl, { y: -6, opacity: 0 }, { y: 0, opacity: 1, duration: .16, ease: 'power2.out' });
    }
  }
  function hidePrioMenu() {
    const menu = $('prioMenu');
    if (menu.classList.contains('hidden') && !prioMenuCard) return;
    menu.classList.add('hidden'); menu.innerHTML = '';
    if (prioMenuCard) { prioMenuCard.classList.remove('prio-ladder'); prioMenuCard = null; }
    prioMenuTodoId = null;
  }
  $('prioMenu').addEventListener('click', (e) => {
    const optEl = e.target.closest('.prio-opt'); if (!optEl) return;
    const t = findTodo(prioMenuTodoId); if (!t) { hidePrioMenu(); return; }
    t.priority = optEl.dataset.prio;
    toast('优先级已改为「' + PRIORITY[t.priority].label + '」');
    hidePrioMenu();
    renderTodos();
  });

  function triggerAutoDone(t) {          // 所有子任务完成 → 父自动完成
    if (!t.done && allChildrenDone(t)) { t.done = true; toast('子任务全部完成，父待办已自动完成 🎉'); }
  }

  // ── 待办编辑弹窗（创建 / 子任务 / 提醒 / 编辑 共用） ──
  let todoEditorCtx = null;   // { mode, todoId?, parentId?, prefillText?, defaultDate?, allowPast? }
  let editorTags = [];        // 编辑中的标签副本（保存时写回）
  let editorTagShake = null, editorTagShakeTimer = null;   // 标签删除抖动确认

  function openTodoEditor(ctx) {
    todoEditorCtx = ctx;
    const t = ctx.todoId ? findTodo(ctx.todoId) : null;
    const parent = ctx.parentId ? findTodo(ctx.parentId) : null;
    const remindOnly = ctx.mode === 'remind';   // 提醒模式：仅可改提醒时间
    const dueOnly = ctx.mode === 'due';         // 完成时间模式：仅可改完成时间
    const lockAll = remindOnly || dueOnly;      // 两种聚焦模式均锁定其余字段

    const titles = { create: '＋ 新建待办', child: '＋ 添加子任务', remind: '⏰ 设置/更改提醒', due: '📅 修改完成时间', edit: '✏️ 编辑待办' };
    $('todoEditorTitle').textContent = titles[ctx.mode] || '待办';
    $('todoEditorText').value = ctx.prefillText || (t ? t.text : '');
    $('todoEditorText').readOnly = lockAll;

    // 完成时间（必填）：创建默认今天 + 1 小时；子任务默认父级完成时间（日期上限锁定为父级）；
    // 已完成的父待办新建子任务 → 默认 1 小时后（父级完成时间必然已过，子任务恢复为可安排的未来时间）
    const dateEl = $('todoEditorDate'), dueEl = $('todoEditorDueTime');
    if (ctx.mode === 'create') {
      dateEl.value = ctx.defaultDate || todayStr();
      dueEl.value = minutesFromNow(60);
    } else {
      dateEl.value = t ? t.date : (parent ? (parent.done ? todayStr() : parent.date) : todayStr());
      dueEl.value = t ? (t.dueTime || '') : (parent ? (parent.done ? minutesFromNow(60) : (parent.dueTime || '')) : minutesFromNow(60));
    }
    if (ctx.mode === 'child' && parent && !parent.done) { dateEl.max = parent.date; }
    else { dateEl.removeAttribute('max'); }
    if ((ctx.mode === 'create' || ctx.mode === 'child') && !ctx.allowPast) { dateEl.min = todayStr(); }
    else { dateEl.removeAttribute('min'); }

    // 提醒（选填）：完整日期 + 时间；留空 = 默认三次提醒
    $('todoEditorRemindDate').value = (t && t.remind) ? t.remind.date : '';
    $('todoEditorRemindTime').value = (t && t.remind) ? t.remind.time : '';

    $('todoEditorPrio').value = t ? t.priority : (parent ? parent.priority : 'medium');

    // 标签（≤3 个 · 每个 ≤10 字）与备注（≤200 字）
    editorTags = (t && t.tags) ? t.tags.slice() : [];
    editorTagShake = null; clearTimeout(editorTagShakeTimer);
    renderEditorTags();
    $('todoEditorTagInput').value = '';
    $('todoEditorRemark').value = (t && t.remark) || '';
    updateRemarkCount();

    // 聚焦模式（提醒/完成时间）：其余字段禁用
    $('todoEditorDate').disabled = remindOnly;
    $('todoEditorDueTime').disabled = remindOnly;
    $('todoEditorRemindDate').disabled = dueOnly;
    $('todoEditorRemindTime').disabled = dueOnly;
    $('todoEditorPrio').disabled = lockAll;
    $('todoEditorTagInput').disabled = lockAll;
    $('todoEditorRemark').disabled = lockAll;
    $('teTagsRow').style.display = lockAll ? 'none' : 'flex';
    $('todoEditorPrioWrap').style.display = lockAll ? 'none' : 'flex';

    $('todoEditorHint').textContent =
      ctx.mode === 'child' ? `子任务完成时间不能晚于父待办（${parent.date} ${parent.dueTime}）`
      : ctx.mode === 'remind'
        ? ((t && t.remind) ? `已设提醒：${t.remind.date} ${t.remind.time}（清空两栏保存 = 恢复默认提醒）` : '未设置提醒：默认在完成前30分钟、前5分钟、完成时各提醒一次')
      : ctx.mode === 'due'
        ? '完成时间决定列表排序与逾期判定；子任务不能晚于父待办'
      : ctx.mode === 'create'
        ? (ctx.defaultDate ? `完成时间默认 1 小时后，将归入 ${ctx.defaultDate}${ctx.allowPast ? '（历史日期补录）' : ''}` : '完成时间默认 1 小时后，可修改；提醒留空 = 默认三次提醒')
      : '完成时间、任务内容必填；提醒留空 = 默认三次提醒';

    $('todoEditorOverlay').classList.remove('hidden');
    modalOpened();
    gsap.fromTo('#todoEditorModal', { scale: .94, opacity: 0 }, { scale: 1, opacity: 1, duration: .2, ease: 'power2.out' });
    setTimeout(() => (ctx.mode === 'remind' ? $('todoEditorRemindTime') : ctx.mode === 'due' ? $('todoEditorDueTime') : $('todoEditorText')).focus(), 80);
  }

  // ── 编辑弹窗内的标签管理（参考笔记标签设计：✕ 抖动二次确认） ──
  const TODO_TAG_MAX = 3, TODO_TAG_LEN = 10;
  function renderEditorTags() {
    const box = $('todoEditorTags');
    if (!editorTags.length) { box.innerHTML = '<span class="te-tags-empty">无标签</span>'; return; }
    box.innerHTML = editorTags.map(tg => `
      <span class="tag-chip ${editorTagShake === tg ? 'shaking' : ''}" title="#${escapeAttr(tg)}">
        <span class="tag-name">${escapeAttr(tg)}</span>
        <em class="tag-del" data-edtag="${escapeAttr(tg)}" title="${editorTagShake === tg ? '再次点击确认删除' : '删除该标签'}">✕</em>
      </span>`).join('');
  }
  function addEditorTag() {
    const v = $('todoEditorTagInput').value.trim();
    if (!v) return;
    if (editorTags.length >= TODO_TAG_MAX) { toast('最多 ' + TODO_TAG_MAX + ' 个标签'); return; }
    if (editorTags.includes(v)) { toast('该标签已存在'); return; }
    if (v.length > TODO_TAG_LEN) { toast('标签最多 ' + TODO_TAG_LEN + ' 个字'); return; }
    editorTags.push(v);
    $('todoEditorTagInput').value = '';
    editorTagShake = null; clearTimeout(editorTagShakeTimer);
    renderEditorTags();
  }
  $('todoEditorTagInput').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); addEditorTag(); }
  });
  $('todoEditorTags').addEventListener('click', (e) => {
    const del = e.target.closest('[data-edtag]'); if (!del) return;
    const tg = del.dataset.edtag;
    if (editorTagShake !== tg) {
      editorTagShake = tg;
      clearTimeout(editorTagShakeTimer);
      editorTagShakeTimer = setTimeout(() => { editorTagShake = null; renderEditorTags(); }, 3000);
      renderEditorTags();
      toast('再次点击 ✕ 确认删除');
    } else {
      editorTags = editorTags.filter(x => x !== tg);
      editorTagShake = null; clearTimeout(editorTagShakeTimer);
      renderEditorTags();
    }
  });
  // 备注字数计数
  function updateRemarkCount() { $('todoEditorRemarkCount').textContent = $('todoEditorRemark').value.length + '/200'; }
  $('todoEditorRemark').addEventListener('input', updateRemarkCount);
  function closeTodoEditor() {
    gsap.to('#todoEditorModal', { scale: .96, opacity: 0, duration: .15, ease: 'power2.in',
      onComplete: () => { $('todoEditorOverlay').classList.add('hidden'); todoEditorCtx = null; modalClosed(); } });
  }
  function saveTodoEditor() {
    const ctx = todoEditorCtx; if (!ctx) return;
    const text = $('todoEditorText').value.trim();
    const date = $('todoEditorDate').value || todayStr();
    const dueTime = $('todoEditorDueTime').value;
    const rd = $('todoEditorRemindDate').value, rt = $('todoEditorRemindTime').value;
    const remind = (rd && rt) ? { date: rd, time: rt } : null;   // 留空 = 默认三次提醒
    const prio = $('todoEditorPrio').value;
    const remark = $('todoEditorRemark').value.trim().slice(0, 200);

    // 校验：完成时间、任务内容必填；提醒两栏需成对出现
    if (!dueTime) { toast('完成时间必填'); return; }
    if ((rd || rt) && !(rd && rt)) { toast('提醒日期与提醒时间需同时设置'); return; }
    // 创建/子任务：完成时间不早于当前（双保险，防手动绕过 min 属性）；归档页补录历史日期除外
    if ((ctx.mode === 'create' || ctx.mode === 'child') && !ctx.allowPast) {
      if (date < todayStr()) { toast('完成日期不能早于当前日期'); return; }
      if (date === todayStr() && dueTime <= nowTimeStr()) { toast('完成时间不能早于当前时间'); return; }
      if (remind && (remind.date < todayStr() || (remind.date === todayStr() && remind.time <= nowTimeStr()))) {
        toast('提醒时间不能早于当前时间'); return;
      }
    }
    // 子任务校验：完成时间不能晚于父待办（已完成父待办重开场景除外——其完成时间必然已过）
    if (ctx.mode === 'child') {
      const parent = findTodo(ctx.parentId);
      if (parent && !parent.done && dueKey({ date, dueTime }) > dueKey(parent)) { toast('子任务的完成时间不能晚于父待办'); return; }
    }
    const rerenderAll = renderTodos;

    if (ctx.mode === 'create') {
      if (!text) { toast('任务内容必填'); return; }
      todos.unshift({ id: Date.now(), text, done: false, priority: prio, date, dueTime, remind, repeat: null, tags: editorTags.slice(), remark, children: [] });
      toast('待办已创建（' + (date === todayStr() ? '今天' : date) + ' ' + dueTime + '）✔');
    }
    if (ctx.mode === 'child') {
      if (!text) { toast('任务内容必填'); return; }
      const parent = findTodo(ctx.parentId);
      if (parent.children.length >= MAX_CHILDREN) { toast('最多 ' + MAX_CHILDREN + ' 个子任务'); return; }
      parent.children.push({ id: Date.now(), text, done: false, priority: prio, date, dueTime, remind, repeat: null, tags: editorTags.slice(), remark, children: [] });
      // 已完成的父待办新增未完成子任务 → 系统判定恢复为未完成
      let msg = '子任务已创建 ✔';
      if (parent.done) { parent.done = false; parent.doneAt = null; msg = '子任务已创建，父待办已自动恢复为未完成 ✔'; }
      toast(msg);
    }
    if (ctx.mode === 'remind') {
      const t = findTodo(ctx.todoId);
      t.remind = remind;
      toast(remind ? '提醒已设为 ' + remind.date + ' ' + remind.time : '已清除提醒（使用默认：完成前30分钟/前5分钟/完成时）');
    }
    if (ctx.mode === 'due') {
      const t = findTodo(ctx.todoId);
      // 子任务：修改后的完成时间不能晚于父待办
      const parent = todos.find(p => p.children.includes(t));
      if (parent && dueKey({ date, dueTime }) > dueKey(parent)) {
        toast('子任务的完成时间不能晚于父待办（' + parent.date + ' ' + parent.dueTime + '）');
        return;
      }
      t.date = date; t.dueTime = dueTime;
      toast('完成时间已改为 ' + date + ' ' + dueTime);
    }
    if (ctx.mode === 'edit') {
      const t = findTodo(ctx.todoId);
      if (!text) { toast('任务内容必填'); return; }
      t.text = text; t.date = date; t.dueTime = dueTime; t.remind = remind; t.priority = prio;
      t.tags = editorTags.slice(); t.remark = remark;
      toast('已保存修改 ✔');
    }
    closeTodoEditor(); rerenderAll();
  }
  $('todoEditorSave').addEventListener('click', saveTodoEditor);
  $('todoEditorCancel').addEventListener('click', closeTodoEditor);
  $('todoEditorClose').addEventListener('click', closeTodoEditor);
  $('todoEditorOverlay').addEventListener('click', (e) => { if (e.target === e.currentTarget) closeTodoEditor(); });
  $('todoEditorText').addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); saveTodoEditor(); } });

  // 事件委托（面板与归档共用）
  function bindTodoList(el, viewDateGetter) {
    el.addEventListener('click', (e) => {
      const btn = e.target.closest('[data-todoact]'); if (!btn || btn.disabled) return;
      const item = e.target.closest('.todo-item'); if (!item) return;
      const t = findTodo(Number(item.dataset.id)); if (!t) return;
      const act = btn.dataset.todoact;
      const viewDate = viewDateGetter();
      const rerender = renderTodos;

      if (act === 'toggle') {
        if (t.done) { toast('已完成的待办/子任务不允许修改（不能取消完成状态）'); return; }
        t.done = !t.done; t.doneAt = t.done ? Date.now() : null;
        if (t.done) toast('已完成，已移至当日末尾 ✔');
        // 子任务勾选 → 检查父级是否可自动完成
        const parent = todos.find(p => p.children.includes(t));
        if (parent) triggerAutoDone(parent);
        rerender(); return;
      }
      if (act === 'collapse') { t.collapsed = !t.collapsed; rerender(); return; }   // 树折叠/展开
      if (act === 'del') {
        // 不直接删除：条目上方弹出确认框二次确认
        todoDeleteConfirmId = t.id; rerender(); return;
      }
      if (act === 'del-yes') {   // 确认删除
        todos = todos.filter(x => x.id !== t.id);
        for (const p of todos) p.children = p.children.filter(c => c.id !== t.id);
        todoDeleteConfirmId = null; toast('待办已删除'); rerender(); return;
      }
      if (act === 'del-no') { todoDeleteConfirmId = null; rerender(); return; }   // 取消
      // ＋子任务在完成守卫之前：已完成的父待办也允许新建子任务（新建后系统自动恢复为未完成）
      if (act === 'addchild') { openTodoEditor({ mode: 'child', parentId: t.id }); return; }
      if (t.done) { toast('已完成的待办不允许修改'); return; }   // 完成后禁改
      if (act === 'edit') { openTodoEditor({ mode: 'edit', todoId: t.id }); return; }

      if (act === 'remind') { openTodoEditor({ mode: 'remind', todoId: t.id }); return; }
      if (act === 'due') { openTodoEditor({ mode: 'due', todoId: t.id }); return; }   // 点击完成时间徽章修改

      if (act === 'repeat') { showRepeatMenu(t, btn); return; }   // 弹出下拉框选择循环方式
      if (act === 'prio') { showPrioMenu(t, btn); return; }       // 弹出优先级阶梯下拉

    });
  }

  // 面板：锁死当天（归档若开着则带搜索词同步刷新）；数据变化同步侧边栏迷你热力图与日期详情
  function renderTodos() {
    renderTodoList($('todoList'), todayStr());
    const arch = document.getElementById('archiveTodoList');
    if (arch) renderTodoList(arch, archiveViewDate, ($('todoArchiveSearch').value || '').trim());
    renderMiniHeat();
    renderDayDetail();
  }
  let archiveViewDate = todayStr();   // 归档页当前查看日期
  bindTodoList($('todoList'), () => todayStr());           // 面板事件委托（锁死当天）
  bindTodoList($('archive-todos'), () => archiveViewDate); // 归档事件委托（委托到外部容器，内部列表动态渲染）

  $('todoInput').addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.target.value.trim()) {
      const prefill = e.target.value.replace(/^- \[[ x]\] ?/, '');
      e.target.value = '';
      openTodoEditor({ mode: 'create', prefillText: prefill });   // 弹窗：文本框+日期(默认今天)+时间
    }
  });
  $('todoRemindBtn').addEventListener('click', () => showReminder('记得给产品文档补充截图'));

  // 归档页日期切换（默认当天；面板不可切日期）
  function switchTodoDate(offset) {
    if (offset === 0) { archiveViewDate = todayStr(); }
    else {
      const d = new Date(archiveViewDate + 'T00:00:00');
      d.setDate(d.getDate() + offset);
      archiveViewDate = `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
    }
    $('todoDateLabel').textContent = archiveViewDate;
    renderTodos();
  }
  $('todoDatePrev').addEventListener('click', () => switchTodoDate(-1));
  $('todoDateNext').addEventListener('click', () => switchTodoDate(1));
  $('todoDateToday').addEventListener('click', () => switchTodoDate(0));
  $('todoDateLabel').textContent = archiveViewDate;
  // 归档待办搜索：跨全部日期模糊匹配（含子任务文本，含已完成）
  $('todoArchiveSearch').addEventListener('input', (e) => renderTodos());
  // 新增待办事项：默认归入当前查看日（查看历史日期时允许补录）
  $('todoArchiveNew').addEventListener('click', () => {
    openTodoEditor({
      mode: 'create',
      defaultDate: archiveViewDate,
      allowPast: archiveViewDate < todayStr(),
    });
  });

  // ── reminder 右上角提醒卡片 ───────────────────
  function showReminder(content) {
    $('reminderContent').textContent = content;
    $('snoozeSelect').value = '';   // 重置下拉框
    const card = $('reminderCard');
    card.classList.remove('hidden');
    gsap.fromTo(card, { x: 60, opacity: 0 }, { x: 0, opacity: 1, duration: .3, ease: 'back.out(1.5)' });
  }
  function hideReminder() {
    gsap.to($('reminderCard'), { x: 60, opacity: 0, duration: .2, onComplete: () => $('reminderCard').classList.add('hidden') });
  }
  // 左上角图标按钮：关闭（稍后不再提醒）
  $('reminderDismiss').addEventListener('click', () => { hideReminder(); toast('已关闭提醒'); });
  // 下拉框选择下次提醒时间
  $('snoozeSelect').addEventListener('change', (e) => {
    if (!e.target.value) return;
    const label = e.target.options[e.target.selectedIndex].text;
    hideReminder();
    toast('已设置下次提醒：' + label);
  });

  // ── 应用级窗口：Inkling 单窗口（标题恒为项目名），设置/统计/日期详情为侧边栏触发的视图 ──
  let currentArchiveView = 'notes';
  function switchArchiveView(view) {
    currentArchiveView = view;
    hideHeatTip();   // 切换视图时收起热力图悬浮明细
    document.querySelectorAll('.side-item').forEach(x => x.classList.toggle('active', x.dataset.view === view));
    $('sideSettings').classList.toggle('active', view === 'settings');
    $('sideStats').classList.toggle('active', view === 'stats');
    ['notes','clips','todos','settings','stats','day'].forEach(v => {
      const el = $('archive-' + v);
      if (el) el.classList.toggle('hidden', v !== view);
    });
    if (view === 'notes' || view === 'clips') renderArchive();
    if (view === 'todos') renderTodos();
    if (view === 'stats') renderStats();
    if (view === 'day') renderDayDetail();
  }
  function openMainWindow(view) {
    if (view) currentArchiveView = view;
    $('mainWindow').classList.remove('hidden');
    gsap.fromTo('#mainWindow', { scale: .94, opacity: 0 }, { scale: 1, opacity: 1, duration: .22, ease: 'power2.out' });
    switchArchiveView(currentArchiveView);
  }
  function closeAllWindows() { $('mainWindow').classList.add('hidden'); }
  document.querySelectorAll('[data-close]').forEach(el => el.addEventListener('click', () => $(el.dataset.close).classList.add('hidden')));
  // 侧边栏：页签 + 底部左侧偏好设置 / 右侧统计
  document.querySelectorAll('.side-item').forEach(x => x.addEventListener('click', () => switchArchiveView(x.dataset.view)));
  $('sideSettings').addEventListener('click', () => switchArchiveView('settings'));
  $('sideStats').addEventListener('click', () => switchArchiveView('stats'));
  // 归档页笔记/粘贴板搜索
  $('noteArchiveSearch').addEventListener('input', renderArchive);
  $('clipArchiveSearch').addEventListener('input', renderArchive);
  // 备注展示样式（设置页配置）：混合 / 图标徽章 / 文本行
  $('settingRemarkStyle').addEventListener('change', renderTodos);

  // ── 侧边栏折叠 / 拖宽 ──
  // 展开（110~280px，默认 150）↔ 图标窄栏（52px）；拖动分隔条实时调宽，低于阈值自动折叠；展开恢复默认宽度
  const SIDE_DEFAULT = 150, SIDE_MIN = 110, SIDE_RAIL = 52, SIDE_MAX = 280;
  const sideEl = document.getElementById('archiveSide');
  function setSideCollapsed(collapsed) {
    sideEl.classList.toggle('collapsed', collapsed);
    sideEl.style.width = (collapsed ? SIDE_RAIL : SIDE_DEFAULT) + 'px';
    $('sideToggle').textContent = collapsed ? '»' : '«';
    $('sideToggle').title = collapsed ? '展开侧边栏（恢复默认宽度）' : '折叠侧边栏';
  }
  $('sideToggle').addEventListener('click', () => setSideCollapsed(!sideEl.classList.contains('collapsed')));
  const sideResizer = $('sideResizer');
  sideResizer.addEventListener('mousedown', (e) => {
    e.preventDefault();
    sideResizer.classList.add('dragging');
    const left = sideEl.getBoundingClientRect().left;
    const onMove = (ev) => {
      const w = Math.min(SIDE_MAX, Math.max(SIDE_RAIL, ev.clientX - left));
      if (w < SIDE_MIN) { setSideCollapsed(true); }
      else { setSideCollapsed(false); sideEl.style.width = w + 'px'; }
    };
    const onUp = () => {
      sideResizer.classList.remove('dragging');
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  });

  // 历史归档渲染 + 窗口内 tab 切换
  const ICON_CLOSE = '<svg width="11" height="11" viewBox="0 0 12 12" fill="none"><path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>';
  const ICON_PIN = '<svg width="11" height="11" viewBox="0 0 24 24" fill="none"><path d="M9 4h6l1 7 3 3v2H5v-2l3-3 1-7z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/><path d="M12 16v5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>';
  const ICON_EDIT = '<svg width="11" height="11" viewBox="0 0 24 24" fill="none"><path d="M4 20h4l11-11-4-4L4 16v4z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/><path d="M13 7l4 4" stroke="currentColor" stroke-width="1.6"/></svg>';
  const ICON_PASTE = '<svg width="11" height="11" viewBox="0 0 24 24" fill="none"><path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><rect x="8" y="2" width="8" height="4" rx="1" stroke="currentColor" stroke-width="1.6"/></svg>';
  const ICON_LINK = '<svg width="11" height="11" viewBox="0 0 24 24" fill="none"><path d="M14 3h7v7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/><path d="M21 3l-9 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><path d="M19 14v5a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>';

  // ── 标签渲染辅助 ──────────────────────────────
  let expandedNoteId = null;     // 手动展开标签的笔记 id
  let shakeNoteId = null, shakeTimer = null;  // 抖动（二次确认删除）状态

  function renderCardTags(note, expanded, shaking) {
    const tags = note.tags || [];
    // 无标签：置灰占位，不可删除，点击弹标签管理窗
    if (tags.length === 0) {
      return '<span class="a-tags"><span class="tag-empty" data-tagempty="1" title="点击管理标签">无标签</span></span>';
    }
    const shown = expanded ? tags : tags.slice(0, TAG_MAX_SHOW);
    let html = '<span class="a-tags">';
    shown.forEach((t) => {
      const display = t.length > TAG_MAX_LEN ? t.slice(0, TAG_MAX_LEN) + '…' : t;
      html += `<span class="tag-chip ${shaking ? 'shaking' : ''}" data-tag="${escapeAttr(t)}" title="#${escapeAttr(t)}">` +
              `<span class="tag-name">${escapeAttr(display)}</span>` +
              `<em class="tag-del" data-tagdel="${escapeAttr(t)}" title="删除该标签">✕</em></span>`;
    });
    if (!expanded && tags.length > TAG_MAX_SHOW) {
      html += `<span class="tag-more" data-tagmore="+${tags.length - TAG_MAX_SHOW}" title="展开全部标签">+${tags.length - TAG_MAX_SHOW}</span>`;
    }
    html += '<span class="shake-tip">再次点击 ✕ 确认删除</span></span>';
    return html;
  }

  function enterShake(noteId) {
    shakeNoteId = noteId;
    clearTimeout(shakeTimer);
    shakeTimer = setTimeout(exitShake, 3000);  // 3 秒无操作自动退出抖动态
    renderArchive();
  }
  function exitShake() {
    shakeNoteId = null; clearTimeout(shakeTimer);
    renderArchive();
  }

  function renderArchive() {
    const noteFilter = ($('noteArchiveSearch').value || '').trim().toLowerCase();
    const clipFilter = ($('clipArchiveSearch').value || '').trim().toLowerCase();

    // 笔记：置顶优先 + 模糊搜索（正文与标签）
    const sortedNotes = [...notes]
      .sort((a, b) => (b.pinned - a.pinned))
      .filter(n => !noteFilter ||
        n.text.toLowerCase().includes(noteFilter) ||
        (n.tags || []).some(t => t.toLowerCase().includes(noteFilter)));
    $('noteArchList').innerHTML = sortedNotes.map(n => {
      const expanded = (n.id === expandedNoteId) || (n.id === shakeNoteId);  // 抖动时也展开全部
      const shaking = (n.id === shakeNoteId);
      return `
      <div class="archive-item ${n.pinned ? 'pinned' : ''} ${shaking ? 'shaking' : ''}" data-id="${n.id}">
        ${noteConfirmId === n.id ? `<div class="card-confirm">
          <span class="card-confirm-text">确认删除该笔记？</span>
          <button class="btn tiny danger" data-act="close-yes">删除</button>
          <button class="btn tiny ghost" data-act="close-no">取消</button>
        </div>` : ''}
        <button class="card-close" data-act="close" title="删除该笔记">${ICON_CLOSE}</button>
        <div class="a-text">${renderMdCard(n.text)}</div>
        <div class="a-meta">
          <span>${n.pinned ? '📌 ' : ''}${n.time}</span>
          ${renderCardTags(n, expanded, shaking)}
          <span class="a-ops">
            <button class="icon-btn ${n.pinned ? 'active-pin' : ''}" data-act="pin" title="${n.pinned ? '取消置顶' : '置顶'}">${ICON_PIN}</button>
            <button class="icon-btn" data-act="edit" title="编辑（回显到主面板）">${ICON_EDIT}</button>
          </span>
        </div>
      </div>`;
    }).join('') || `<div class="todo-empty">${noteFilter ? '未找到匹配的笔记' : '暂无笔记'}</div>`;

    // 粘贴板：置顶优先 + 模糊搜索
    const TYPE_LABEL = { text: '文本', link: '链接', code: '代码', image: '图片', richtext: '富文本' };
    const TEXT_EDITABLE = ['text', 'link', 'code', 'richtext'];   // 可文本编辑的类型（图片等不支持）
    const sortedClips = [...clips]
      .sort((a, b) => (b.pinned - a.pinned))
      .filter(c => !clipFilter || c.text.toLowerCase().includes(clipFilter));
    $('clipArchList').innerHTML = sortedClips.map(c => `
      <div class="archive-item clip-arch ${c.pinned ? 'pinned' : ''}" data-id="${c.id}">
        ${clipConfirmId === c.id ? `<div class="card-confirm">
          <span class="card-confirm-text">确认删除该条目？</span>
          <button class="btn tiny danger" data-clipact="del-yes">删除</button>
          <button class="btn tiny ghost" data-clipact="del-no">取消</button>
        </div>` : ''}
        <button class="card-close" data-clipact="del" title="删除该条目">${ICON_CLOSE}</button>
        <div class="a-text">${c.text}</div>
        <div class="a-meta">
          <span class="clip-type ${c.type}">${TYPE_LABEL[c.type] || c.type}</span>
          <span>${c.pinned ? '📌 ' : ''}${c.time}</span>
          <span class="a-ops">
            <button class="icon-btn" data-clipact="paste" title="粘贴到鼠标光标处">${ICON_PASTE}</button>
            <button class="icon-btn ${c.pinned ? 'active-pin' : ''}" data-clipact="pin"
              title="${c.pinned ? '取消置顶' : '置顶'}">${ICON_PIN}</button>
            ${c.type === 'link'
              ? `<a class="icon-btn clip-open" href="${c.text}" target="_blank" rel="noopener noreferrer" title="用默认浏览器打开该链接">${ICON_LINK}</a>`
              : ''}
            ${TEXT_EDITABLE.includes(c.type)
              ? `<button class="icon-btn" data-clipact="edit" title="编辑内容（弹框回显修改）">${ICON_EDIT}</button>`
              : ''}
          </span>
        </div>
      </div>`).join('') || `<div class="todo-empty">${clipFilter ? '未找到匹配的条目' : '暂无粘贴板条目'}</div>`;

    // 待办列表随日期/搜索词刷新（日期条为静态结构，不再整体重建）
    renderTodoList(document.getElementById('archiveTodoList'), archiveViewDate, ($('todoArchiveSearch').value || '').trim());
    renderMiniHeat();   // 笔记/粘贴板数据变化同步侧边栏迷你热力图
    renderDayDetail();  // 同步日期详情视图
  }

  // 笔记卡片操作：标签弹窗 / 标签删除抖动确认 / 展开省略 / 右上角关闭 / 置顶 / 编辑（回显）
  $('archive-notes').addEventListener('click', (e) => {
    const item = e.target.closest('.archive-item');
    if (!item) return;
    const id = Number(item.dataset.id);
    const n = notes.find(x => x.id === id); if (!n) return;

    // ① 标签 ✕ 删除按钮（二次确认：抖动 → 再次点击删除）
    const tagDel = e.target.closest('[data-tagdel]');
    if (tagDel) {
      e.stopPropagation();
      const tag = tagDel.dataset.tagdel;
      if (shakeNoteId !== id) {
        enterShake(id);                       // 第一次：进入抖动确认态
        toast('再次点击 ✕ 确认删除该标签');
      } else {
        n.tags = n.tags.filter(t => t !== tag);  // 第二次：执行删除
        toast(`已删除标签 #${tag}`);
        n.tags.length === 0 ? exitShake() : renderArchive();
      }
      return;
    }
    // ② 展开省略的标签
    const more = e.target.closest('[data-tagmore]');
    if (more) { e.stopPropagation(); expandedNoteId = id; renderArchive(); return; }
    // ②.5 置灰「无标签」占位 → 点击弹标签管理窗
    const emptyTag = e.target.closest('[data-tagempty]');
    if (emptyTag) { e.stopPropagation(); openTagManager({ type: 'note', id }); return; }
    // ③ 点击标签 chip 文字 → 打开标签管理弹窗
    const chip = e.target.closest('.tag-chip');
    if (chip) { e.stopPropagation(); openTagManager({ type: 'note', id }); return; }

    // ④ 卡片右上角 ✕（二次确认）/ 置顶 / 编辑
    const btn = e.target.closest('[data-act]'); if (!btn) return;
    const act = btn.dataset.act;
    if (act === 'close') { noteConfirmId = id; renderArchive(); }   // 先确认再删除
    if (act === 'close-yes') {
      notes = notes.filter(x => x.id !== id);
      noteConfirmId = null;
      if (shakeNoteId === id) shakeNoteId = null;
      toast('笔记已删除');
      renderArchive();
    }
    if (act === 'close-no') { noteConfirmId = null; renderArchive(); }
    if (act === 'pin') {
      n.pinned = !n.pinned;
      toast(n.pinned ? '已置顶 📌' : '已取消置顶');
      renderArchive();
      // 置顶时同步弹出桌面小浮窗
      if (n.pinned) { $('pinnedContent').textContent = n.text.slice(0, 50); $('pinnedWindow').classList.remove('hidden'); }
    }
    if (act === 'edit') {
      // 回显：关闭历史窗 → 展开主面板 → 内容填入编辑器进入编辑模式（右下角回显该笔记标签）
      editingNoteId = id;
      closeAllWindows();
      switchMode('note');
      showPanel();
      editor.innerText = n.text;
      renderInlineMarkdown(); renderPanelTags();
      $('btnArchive').textContent = '保存修改 ✓';
      toast('内容已回显，修改后点击「保存修改」');
    }
  });

  // ── 标签管理弹窗（增 / 改 / 删，支持笔记与草稿两种目标） ──
  // tagMgrTarget: { type:'note', id } | { type:'draft' }
  let tagMgrTarget = null;

  // 统一取目标标签数组（草稿直接操作 draftTags 引用）
  function getMgrTags() {
    if (tagMgrTarget?.type === 'note') {
      const n = notes.find(x => x.id === tagMgrTarget.id);
      return n ? n.tags : null;
    }
    return draftTags;
  }
  function openTagManager(target) {
    tagMgrTarget = target;
    mgrShakeIdx = null;   // 打开时重置删除确认态
    $('tagMgrSub').textContent = target.type === 'draft' ? '当前正在编写的念头（未归档）的标签' : '当前笔记的标签';
    renderTagMgrList();
    $('tagManagerOverlay').classList.remove('hidden');
    modalOpened();
    gsap.fromTo('#tagManagerModal', { scale: .94, opacity: 0 }, { scale: 1, opacity: 1, duration: .2, ease: 'power2.out' });
    $('tagAddInput').value = '';
    setTimeout(() => $('tagAddInput').focus(), 80);
  }
  function closeTagManager() {
    gsap.to('#tagManagerModal', { scale: .96, opacity: 0, duration: .15, ease: 'power2.in',
      onComplete: () => {
        $('tagManagerOverlay').classList.add('hidden');
        tagMgrTarget = null; mgrShakeIdx = null;
        modalClosed();
        renderArchive();      // 刷新卡片左下角标签
        renderPanelTags();    // 刷新面板右下角标签
      } });
  }
  function renderTagMgrList() {
    const tags = getMgrTags();
    const list = $('tagMgrList');
    if (!tags || tags.length === 0) {
      list.innerHTML = '<li class="tag-mgr-empty">暂无标签，在上方输入框添加</li>'; return;
    }
    list.innerHTML = tags.map((t, i) => `
      <li class="tag-mgr-item ${mgrShakeIdx === i ? 'shaking' : ''}" data-idx="${i}">
        <span class="tag-mgr-name" contenteditable="true" spellcheck="false" data-mgridx="${i}" title="点击文字修改（最多 ${TAG_MAX_LEN} 字）">${escapeAttr(t)}</span>
        <em class="tag-del mgr-del ${mgrShakeIdx === i ? 'confirm' : ''}" data-mgrdel="${i}" title="${mgrShakeIdx === i ? '再次点击确认删除' : '删除该标签'}">✕</em>
      </li>`).join('');
  }
  // 新增标签
  function tagAdd() {
    const v = $('tagAddInput').value.trim();
    const tags = getMgrTags(); if (!tags) return;
    if (!v) { toast('标签名不能为空'); return; }
    if (tags.includes(v)) { toast('该标签已存在'); return; }
    tags.push(v);
    $('tagAddInput').value = '';
    renderTagMgrList();
    toast('标签已添加');
  }
  $('tagAddBtn').addEventListener('click', tagAdd);
  $('tagAddInput').addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); tagAdd(); } });
  // 删除（抖动二次确认：第一次点击抖动，3 秒内再次点击才删除）
  let mgrShakeIdx = null, mgrShakeTimer = null;
  $('tagMgrList').addEventListener('click', (e) => {
    const del = e.target.closest('[data-mgrdel]'); if (!del) return;
    const tags = getMgrTags(); if (!tags) return;
    const idx = Number(del.dataset.mgrdel);
    if (mgrShakeIdx !== idx) {
      mgrShakeIdx = idx;
      clearTimeout(mgrShakeTimer);
      mgrShakeTimer = setTimeout(() => { mgrShakeIdx = null; renderTagMgrList(); }, 3000);
      renderTagMgrList();
      toast('再次点击 ✕ 确认删除');
    } else {
      tags.splice(idx, 1);
      mgrShakeIdx = null; clearTimeout(mgrShakeTimer);
      renderTagMgrList();
      toast('标签已删除');
    }
  });
  // 修改（contenteditable：Enter 或失焦保存）
  $('tagMgrList').addEventListener('keydown', (e) => {
    const span = e.target.closest('[data-mgridx]'); if (!span) return;
    if (e.key === 'Enter') { e.preventDefault(); span.blur(); }
  });
  $('tagMgrList').addEventListener('blur', (e) => {
    const span = e.target.closest('[data-mgridx]'); if (!span) return;
    const tags = getMgrTags(); if (!tags) return;
    const idx = Number(span.dataset.mgridx);
    const v = span.textContent.trim().slice(0, TAG_MAX_LEN);
    if (!v) { toast('标签名不能为空，已恢复'); span.textContent = tags[idx]; return; }
    if (v !== tags[idx] && tags.includes(v)) { toast('标签名重复，已恢复'); span.textContent = tags[idx]; return; }
    if (v !== tags[idx]) { tags[idx] = v; toast('标签已修改'); }
  }, true);   // blur 不冒泡，需捕获阶段监听
  // 关闭路径
  $('tagManagerClose').addEventListener('click', closeTagManager);
  $('tagManagerOverlay').addEventListener('click', (e) => { if (e.target === e.currentTarget) closeTagManager(); });

  // 归档页粘贴板操作：Pin（置顶）与编辑是两个独立功能
  $('archive-clips').addEventListener('click', (e) => {
    const btn = e.target.closest('[data-clipact]'); if (!btn) return;
    const item = e.target.closest('.archive-item'); if (!item) return;
    const c = clips.find(x => x.id == item.dataset.id); if (!c) return;
    const act = btn.dataset.clipact;

    if (act === 'pin') {
      // 置顶：直接切换，任何类型均可，不弹编辑框
      c.pinned = !c.pinned;
      toast(c.pinned ? '已置顶 📌' : '已取消置顶');
      renderArchive(); renderClips($('clipSearch').value);
    }
    if (act === 'edit') {
      // 编辑：弹出文本编辑浮框，回显内容，保存后替换原文本（不影响置顶状态）
      openClipEditor(c);
    }
    if (act === 'del') { clipConfirmId = c.id; renderArchive(); }   // 先确认再删除
    if (act === 'del-yes') {
      clips.splice(clips.indexOf(c), 1); clipConfirmId = null;
      toast('条目已删除');
      renderArchive(); renderClips($('clipSearch').value);
    }
    if (act === 'del-no') { clipConfirmId = null; renderArchive(); renderClips($('clipSearch').value); }
    if (act === 'paste') {
      pasteAtCursor(c);
    }
  });

  // 双击条目 = 粘贴到鼠标光标处（写入剪贴板并模拟在光标位置粘贴）
  let lastMouse = { x: innerWidth / 2, y: innerHeight / 2 };
  document.addEventListener('mousemove', (e) => { lastMouse = { x: e.clientX, y: e.clientY }; });
  function pasteAtCursor(c) {
    if (navigator.clipboard) navigator.clipboard.writeText(c.text).catch(() => {});
    toast(`已粘贴到鼠标光标处 (${lastMouse.x}, ${lastMouse.y}) ✔`);
  }
  $('archive-clips').addEventListener('dblclick', (e) => {
    const item = e.target.closest('.archive-item'); if (!item) return;
    const c = clips.find(x => x.id == item.dataset.id); if (!c) return;
    pasteAtCursor(c);
  });

  // ── 统计：热力图（月份范围/悬浮明细/逾期红框） + 月度趋势折线图 ──
  // 固定种子伪随机：同一份数据贯穿热力图与折线图，悬浮明细可复现
  function mulberry32(seed) {
    return function () {
      seed |= 0; seed = (seed + 0x6D2B79F5) | 0;
      let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  function buildStatsData() {
    const rand = mulberry32(20260828);
    const days = [];
    const today = new Date(); today.setHours(0, 0, 0, 0);
    const start = new Date(today); start.setDate(start.getDate() - 181);
    start.setDate(start.getDate() - (start.getDay() + 6) % 7);   // 起点对齐到周一
    const cur = new Date(start);
    while (cur <= today) {
      const ds = `${cur.getFullYear()}-${String(cur.getMonth()+1).padStart(2,'0')}-${String(cur.getDate()).padStart(2,'0')}`;
      const weekend = [0, 6].includes(cur.getDay());
      const notes = rand() < .18 ? 0 : Math.round(rand() * (weekend ? 3 : 7));
      const clips = rand() < .12 ? 0 : Math.round(rand() * (weekend ? 6 : 14));
      const todos = rand() < .3 ? 0 : 1 + Math.round(rand() * 4);
      const open = rand() < .38 ? Math.min(todos, 1 + Math.round(rand() * 1.4)) : 0;   // 当日未完成
      const overdue = ds < todayStr() ? open : 0;   // 规则同待办列表：当天未完成即逾期
      days.push({ date: ds, notes, clips, todos, done: todos - open, overdue });
      cur.setDate(cur.getDate() + 1);
    }
    return days;
  }
  const statsData = buildStatsData();
  const statsByDate = Object.fromEntries(statsData.map(d => [d.date, d]));
  const WEEK_CN = ['日', '一', '二', '三', '四', '五', '六'];

  function renderStats() { renderHeatmap(); renderTrend(); }

  function renderHeatmap() {
    const wrap = $('heatmap');
    const STEP = 17;   // 14px 格子 + 3px 间距
    // 顶部月份范围标签：每列（周）检查月份变化，避免标签重叠
    let monthsHtml = '', prevMonth = -1, lastX = -99;
    statsData.forEach((d, i) => {
      if (i % 7 !== 0) return;
      const m = Number(d.date.slice(5, 7));
      const x = Math.floor(i / 7) * STEP;
      if (m !== prevMonth && x - lastX >= STEP * 3) {
        monthsHtml += `<span style="left:${x}px">${m}月</span>`;
        prevMonth = m; lastX = x;
      }
    });
    const weekdayHtml = ['一', '', '', '四', '', '', '日'].map(s => `<span>${s}</span>`).join('');
    const cellsHtml = statsData.map(d => {
      const total = d.notes + d.clips + d.todos;
      const lv = total === 0 ? 0 : total < 5 ? 1 : total < 10 ? 2 : total < 18 ? 3 : 4;
      const bg = lv ? ` style="background:rgba(108,140,255,${[0, .18, .38, .6, .88][lv]})"` : '';
      return `<div class="heat-cell ${d.overdue > 0 ? 'ovd' : ''}" data-date="${d.date}"${bg}></div>`;
    }).join('');
    wrap.innerHTML =
      `<div class="heat-months">${monthsHtml}</div>
       <div class="heat-flex">
         <div class="heat-weekdays">${weekdayHtml}</div>
         <div class="heat-grid">${cellsHtml}</div>
       </div>
       <div class="heat-legend">少
         <i style="background:rgba(108,140,255,.18)"></i><i style="background:rgba(108,140,255,.38)"></i><i style="background:rgba(108,140,255,.6)"></i><i style="background:rgba(108,140,255,.88)"></i>
         多 <i class="lg-ovd" style="background:rgba(108,140,255,.3)"></i> 存在逾期</div>`;
  }

  // 悬浮明细：日期 + 笔记/复制项/待办（已完成、逾期），存在逾期时 tooltip 红边
  // 数据源：统计页大热力图用模拟数据；侧边栏当月迷你热力图（mh-cell）用真实 pin 数据
  function showHeatTip(cell) {
    const source = cell.classList.contains('mh-cell') ? realByDate : statsByDate;
    const d = source[cell.dataset.date]; if (!d) return;
    const tip = $('heatTip');
    const dt = new Date(d.date + 'T00:00:00');
    tip.innerHTML = `<div class="tip-title">${d.date} 周${WEEK_CN[dt.getDay()]}</div>
      <div class="tip-row">📝 笔记 <b>${d.notes}</b> 条</div>
      <div class="tip-row">📋 复制项 <b>${d.clips}</b> 条</div>
      <div class="tip-row">✅ 待办 <b>${d.todos}</b> 条 · 已完成 <b>${d.done}</b>${d.overdue > 0 ? ` · <span class="ovd-red">逾期 ${d.overdue}</span>` : ''}</div>`;
    tip.classList.toggle('ovd', d.overdue > 0);
    tip.classList.remove('hidden');
    const r = cell.getBoundingClientRect();
    const tw = tip.offsetWidth, th = tip.offsetHeight;
    let top = r.top - th - 8;
    if (top < 8) top = r.bottom + 8;
    let left = Math.min(Math.max(8, r.left + r.width / 2 - tw / 2), innerWidth - tw - 8);
    tip.style.top = top + 'px'; tip.style.left = left + 'px';
    gsap.fromTo(tip, { opacity: 0, y: 4 }, { opacity: 1, y: 0, duration: .15, overwrite: true });
  }
  function hideHeatTip() { $('heatTip').classList.add('hidden'); }
  $('heatmap').addEventListener('mouseover', (e) => { const c = e.target.closest('.heat-cell'); if (c) showHeatTip(c); });
  $('heatmap').addEventListener('mouseout', (e) => { if (e.target.closest('.heat-cell')) hideHeatTip(); });
  $('heatmap').addEventListener('mouseleave', hideHeatTip);

  // 近 6 个月趋势：SVG 折线图（笔记/粘贴板/待办）
  function renderTrend() {
    const el = $('trendChart');
    const byMonth = {};
    statsData.forEach(d => {
      const key = d.date.slice(0, 7);
      (byMonth[key] = byMonth[key] || { note: 0, clip: 0, todo: 0 });
      byMonth[key].note += d.notes; byMonth[key].clip += d.clips; byMonth[key].todo += d.todos;
    });
    const keys = Object.keys(byMonth).sort().slice(-6);
    const series = [
      { key: 'note', label: '笔记', color: '#ff8a8a' },
      { key: 'clip', label: '粘贴板', color: '#ffd76e' },
      { key: 'todo', label: '待办', color: '#7ee0a8' },
    ];
    const W = 620, H = 190, P = { l: 40, r: 12, t: 24, b: 28 };
    const iw = W - P.l - P.r, ih = H - P.t - P.b;
    const maxV = Math.max(10, ...keys.flatMap(k => series.map(s => byMonth[k][s.key])));
    const niceMax = Math.ceil(maxV / 10) * 10;
    const x = (i) => P.l + (keys.length === 1 ? iw / 2 : i * iw / (keys.length - 1));
    const y = (v) => P.t + ih - (v / niceMax) * ih;
    let svg = `<svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet" role="img" aria-label="月度趋势折线图">`;
    for (let g = 0; g <= 3; g++) {   // 网格线与刻度
      const v = niceMax * g / 3, yy = y(v);
      svg += `<line x1="${P.l}" y1="${yy}" x2="${W - P.r}" y2="${yy}" stroke="rgba(255,255,255,.09)"/>`;
      svg += `<text x="${P.l - 8}" y="${yy + 3.5}" text-anchor="end" font-size="10" fill="rgba(255,255,255,.45)">${Math.round(v)}</text>`;
    }
    series.forEach(s => {            // 折线 + 数据点（悬浮显示数值）
      const pts = keys.map((k, i) => `${x(i)},${y(byMonth[k][s.key])}`).join(' ');
      svg += `<polyline points="${pts}" fill="none" stroke="${s.color}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`;
      keys.forEach((k, i) => {
        svg += `<circle cx="${x(i)}" cy="${y(byMonth[k][s.key])}" r="3.2" fill="${s.color}"><title>${k} · ${s.label}：${byMonth[k][s.key]}</title></circle>`;
      });
    });
    keys.forEach((k, i) => {         // 月份标签
      svg += `<text x="${x(i)}" y="${H - 8}" text-anchor="middle" font-size="10.5" fill="rgba(255,255,255,.55)">${Number(k.slice(5))}月</text>`;
    });
    svg += '</svg>';
    el.innerHTML = `<div class="trend-legend">${series.map(s => `<span><i style="background:${s.color}"></i>${s.label}</span>`).join('')}</div>` + svg;
  }

  // ── 侧边栏当月迷你热力图 + 日期详情查询（真实 pin 数据） ──
  let realByDate = {};        // 真实数据按日统计（迷你热力图数据源）
  let dayDetailDate = null;   // 日期详情当前查看日
  let dayFilter = 'all';      // 类别筛选：all | note | clip | todo
  let daySearch = '';         // 日期详情关键字搜索

  function buildRealByDate() {
    const map = {};
    const bucket = (date) => (map[date] = map[date] || { date, notes: 0, clips: 0, todos: 0, done: 0, overdue: 0 });
    notes.forEach(n => { if (n.date) bucket(n.date).notes++; });
    clips.forEach(c => { if (c.date) bucket(c.date).clips++; });
    const walk = (t) => {
      if (!t.date) return;
      const d = bucket(t.date);
      d.todos++;
      if (t.done) d.done++;
      else if (isOverdue(t, todayStr())) d.overdue++;
    };
    todos.forEach(t => { walk(t); t.children.forEach(walk); });
    return map;
  }

  function renderMiniHeat() {
    realByDate = buildRealByDate();
    const box = $('miniHeat'); if (!box) return;
    // 侧边栏分类计数徽章（待办含子任务）
    const cn = $('countNotes'), cc = $('countClips'), ct = $('countTodos');
    if (cn) cn.textContent = notes.length;
    if (cc) cc.textContent = clips.length;
    if (ct) { let n = 0; todos.forEach(t => n += 1 + t.children.length); ct.textContent = n; }
    const now = new Date();
    const y = now.getFullYear(), m = now.getMonth();
    const pad2 = (n) => String(n).padStart(2, '0');
    const lead = (new Date(y, m, 1).getDay() + 6) % 7;   // 周一对齐的月初空位
    const daysInMonth = new Date(y, m + 1, 0).getDate();
    const cols = Math.ceil((lead + daysInMonth) / 7);
    let cells = '';
    for (let i = 0; i < cols * 7; i++) {
      const dayNum = i - lead + 1;
      if (dayNum < 1 || dayNum > daysInMonth) { cells += '<i class="mh-blank"></i>'; continue; }
      const ds = `${y}-${pad2(m + 1)}-${pad2(dayNum)}`;
      const d = realByDate[ds];
      const total = d ? d.notes + d.clips + d.todos : 0;
      const lv = total === 0 ? 0 : total < 3 ? 1 : total < 6 ? 2 : total < 10 ? 3 : 4;
      const bg = lv ? ` style="background:rgba(108,140,255,${[0, .18, .38, .6, .88][lv]})"` : '';
      cells += `<span class="heat-cell mh-cell ${d && d.overdue > 0 ? 'ovd' : ''} ${ds === dayDetailDate ? 'selected' : ''}" data-date="${ds}"${bg}></span>`;
    }
    box.innerHTML = `<div class="mh-title">${m + 1}月活跃（悬浮明细 · 点击查当日）</div><div class="mh-grid">${cells}</div>`;
  }

  // 点击某日 → 主内容区展示该日全部 pin（时间升序；待办取完成时间）
  function openDayDetail(dateStr) {
    dayDetailDate = dateStr;
    renderMiniHeat();
    switchArchiveView('day');
  }

  const timeOf = (s) => { const m = String(s || '').match(/(\d{1,2}):(\d{2})/); return m ? `${m[1].padStart(2, '0')}:${m[2]}` : '00:00'; };

  function renderDayDetail() {
    const listEl = $('dayDetailList'); if (!listEl) return;
    if (!dayDetailDate) { listEl.innerHTML = '<div class="todo-empty">在左侧当月热力图上选择一个日期</div>'; return; }
    $('dayDetailDateLabel').textContent = dayDetailDate + (dayDetailDate === todayStr() ? ' · 今天' : '');
    // 关键字过滤（匹配文本/标签/备注）
    const q = (daySearch || '').trim().toLowerCase();
    const matchQ = (it) => !q || it.hay.toLowerCase().includes(q);
    // 收集该日全部 pin（带完整详情）
    const items = [];
    if (dayFilter === 'all' || dayFilter === 'note') {
      notes.filter(n => n.date === dayDetailDate).forEach(n => items.push({
        type: 'note', id: n.id, time: timeOf(n.time),
        titleHtml: renderMdCard(n.text), rawText: n.text,
        hay: n.text + ' ' + (n.tags || []).join(' '), tags: n.tags || [],
      }));
    }
    if (dayFilter === 'all' || dayFilter === 'clip') {
      const TYPE_LABEL = { text: '文本', link: '链接', code: '代码', image: '图片', richtext: '富文本' };
      clips.filter(c => c.date === dayDetailDate).forEach(c => items.push({
        type: 'clip', id: c.id, time: timeOf(c.time),
        titleHtml: escapeHtml(c.text), rawText: c.text, hay: c.text,
        clipType: c.type, clipLabel: TYPE_LABEL[c.type] || c.type,
      }));
    }
    if (dayFilter === 'all' || dayFilter === 'todo') {
      const walk = (t, isChild, parentText) => {
        if (t.date === dayDetailDate) items.push({
          type: 'todo', id: t.id, time: t.dueTime,
          titleHtml: escapeHtml(t.text), rawText: t.text,
          hay: t.text + ' ' + (t.tags || []).join(' ') + ' ' + (t.remark || ''),
          done: t.done, overdue: isOverdue(t, dayDetailDate) && !t.done, isChild, parentText,
          tags: t.tags || [], remark: t.remark || '', remind: t.remind, repeat: t.repeat, priority: t.priority,
        });
        t.children.forEach(c => walk(c, true, t.text));
      };
      todos.forEach(t => walk(t, false));
    }
    items.sort((a, b) => a.time.localeCompare(b.time));   // 按时间先后（待办取完成时间）
    const TYPE = { note: '📝 笔记', clip: '📋 粘贴板', todo: '✅ 待办' };
    const KIND_NAME = { note: '笔记', clip: '条目', todo: '待办事项' };
    const confirmIdFor = (it) => it.type === 'note' ? noteConfirmId : it.type === 'clip' ? clipConfirmId : todoDeleteConfirmId;
    const visible = items.filter(matchQ);
    listEl.innerHTML = visible.map(it => {
      const confirmed = confirmIdFor(it) === it.id;
      const remindTxt = it.remind
        ? `⏰ ${it.remind.date === dayDetailDate ? it.remind.time : it.remind.date + ' ' + it.remind.time}`
        : '';
      return `
      <div class="day-item ${it.type} ${it.done ? 'done' : ''}" data-kind="${it.type}" data-id="${it.id}">
        ${confirmed ? `<div class="card-confirm">
          <span class="card-confirm-text">确认删除该${KIND_NAME[it.type]}？</span>
          <button class="btn tiny danger" data-dayact="del-yes">删除</button>
          <button class="btn tiny ghost" data-dayact="del-no">取消</button>
        </div>` : ''}
        <span class="day-time">${it.time}</span>
        <span class="day-badge ${it.type}">${TYPE[it.type]}</span>
        <div class="day-body">
          <div class="day-title-row">
            ${it.type === 'todo' ? `<span class="prio-badge ${it.priority}">${PRIORITY[it.priority].label}</span>` : ''}
            ${it.clipLabel ? `<span class="clip-type ${it.clipType}">${it.clipLabel}</span>` : ''}
            <div class="day-text ${it.type === 'note' ? 'd-clamp4' : 'd-clamp3'}" title="${escapeAttr(it.rawText)}">${it.titleHtml}</div>
            ${it.overdue ? '<span class="day-overdue">逾期</span>' : ''}
          </div>
          ${(remindTxt || it.repeat) ? `<div class="day-meta">
            ${it.remind ? `<span class="todo-meta">${remindTxt}</span>` : ''}
            ${it.repeat ? `<span class="todo-meta">🔁 ${it.repeat === 'daily' ? '每天重复' : '每周重复'}</span>` : ''}
          </div>` : ''}
          ${(it.tags && it.tags.length) ? `<div class="day-tags">${it.tags.map(tg =>
            `<span class="tag-chip todo-tag"><span class="tag-name">${escapeAttr(tg)}</span></span>`).join('')}</div>` : ''}
          ${it.remark ? `<div class="day-remark" title="${escapeAttr(it.remark)}">${escapeHtml(it.remark)}</div>` : ''}
          ${it.isChild ? `<div class="day-sub">子任务 · 属于「${escapeHtml(it.parentText)}」</div>` : ''}
        </div>
        <div class="day-ops">
          <button class="icon-btn" data-dayact="edit" title="编辑">${ICON_EDIT}</button>
          <button class="icon-btn" data-dayact="del" title="删除">${ICON_CLOSE}</button>
        </div>
      </div>`;
    }).join('') || `<div class="todo-empty">${q ? `未找到匹配「${escapeHtml(daySearch.trim())}」的记录` : '该日暂无' + (dayFilter === 'all' ? '记录' : { note: '笔记', clip: '粘贴板条目', todo: '待办事项' }[dayFilter])}</div>`;
  }

  // 日期详情卡片操作：编辑 / 删除（二次确认）
  $('dayDetailList').addEventListener('click', (e) => {
    const btn = e.target.closest('[data-dayact]'); if (!btn) return;
    const card = e.target.closest('.day-item'); if (!card) return;
    const kind = card.dataset.kind, id = Number(card.dataset.id);
    const act = btn.dataset.dayact;
    const rerenderDay = () => { renderMiniHeat(); renderDayDetail(); };

    if (act === 'edit') {
      if (kind === 'note') {
        const n = notes.find(x => x.id === id); if (!n) return;
        editingNoteId = id; switchMode('note'); showPanel();
        editor.innerText = n.text; renderInlineMarkdown(); renderPanelTags();
        $('btnArchive').textContent = '保存修改 ✓';
        toast('内容已回显，修改后点击「保存修改」');
      } else if (kind === 'clip') {
        const c = clips.find(x => x.id === id); if (c) openClipEditor(c);
      } else if (kind === 'todo') {
        if (findTodo(id)?.done) { toast('已完成的待办不允许修改'); return; }
        openTodoEditor({ mode: 'edit', todoId: id });
      }
      return;
    }
    if (act === 'del') {
      if (kind === 'note') noteConfirmId = id;
      else if (kind === 'clip') clipConfirmId = id;
      else todoDeleteConfirmId = id;
      renderDayDetail(); return;
    }
    if (act === 'del-yes') {
      if (kind === 'note') {
        notes = notes.filter(x => x.id !== id); noteConfirmId = null;
        toast('笔记已删除'); renderArchive();
      } else if (kind === 'clip') {
        const c = clips.find(x => x.id === id); if (c) clips.splice(clips.indexOf(c), 1);
        clipConfirmId = null; toast('条目已删除');
        renderArchive(); renderClips($('clipSearch').value);
      } else {
        todos = todos.filter(x => x.id !== id);
        for (const p of todos) p.children = p.children.filter(c => c.id !== id);
        todoDeleteConfirmId = null; toast('待办已删除'); renderTodos();
      }
      rerenderDay(); return;
    }
    if (act === 'del-no') {
      noteConfirmId = null; clipConfirmId = null; todoDeleteConfirmId = null;
      renderDayDetail(); return;
    }
  });
  // 日期详情关键字搜索
  $('dayDetailSearch').addEventListener('input', (e) => { daySearch = e.target.value; renderDayDetail(); });

  $('miniHeat').addEventListener('click', (e) => {
    const cell = e.target.closest('.mh-cell'); if (!cell) return;
    openDayDetail(cell.dataset.date);
  });
  $('miniHeat').addEventListener('mouseover', (e) => { const c = e.target.closest('.mh-cell'); if (c) showHeatTip(c); });
  $('miniHeat').addEventListener('mouseout', (e) => { if (e.target.closest('.mh-cell')) hideHeatTip(); });
  $('miniHeat').addEventListener('mouseleave', hideHeatTip);
  // 日期详情类别筛选
  $('dayFilters').addEventListener('click', (e) => {
    const chip = e.target.closest('.day-filter'); if (!chip) return;
    dayFilter = chip.dataset.filter;
    document.querySelectorAll('.day-filter').forEach(x => x.classList.toggle('active', x === chip));
    renderDayDetail();
  });

  // ── 托盘：左键历史归档 / 右键菜单 ─────────────
  $('trayIcon').addEventListener('click', () => { $('trayMenu').classList.add('hidden'); openMainWindow('notes'); });
  $('trayIcon').addEventListener('contextmenu', (e) => {
    e.preventDefault();
    const menu = $('trayMenu');
    menu.classList.toggle('hidden');
    menu.style.top = '32px'; menu.style.right = '10px';
  });
  document.addEventListener('click', (e) => { if (!e.target.closest('#trayMenu,#trayIcon')) $('trayMenu').classList.add('hidden'); });
  document.querySelectorAll('#trayMenu .menu-row').forEach(row => row.addEventListener('click', () => {
    $('trayMenu').classList.add('hidden');
    const a = row.dataset.action;
    if (a === 'history') openMainWindow('notes');
    if (a === 'settings') openMainWindow('settings');
    if (a === 'stats') openMainWindow('stats');
    if (a === 'quit') toast('原型中无法真的退出 🙂');
  }));

  // ── pinned 置顶浮窗：拖拽/透明度/双击编辑 ─────
  const pinned = $('pinnedWindow');
  let drag = null;
  pinned.addEventListener('mousedown', (e) => {
    if (e.target.closest('input,.pinned-close')) return;
    drag = { x: e.clientX - pinned.offsetLeft, y: e.clientY - pinned.offsetTop };
  });
  document.addEventListener('mousemove', (e) => {
    if (!drag) return;
    pinned.style.left = (e.clientX - drag.x) + 'px';
    pinned.style.top = (e.clientY - drag.y) + 'px';
    pinned.style.right = 'auto'; pinned.style.bottom = 'auto';
  });
  document.addEventListener('mouseup', () => drag = null);
  $('pinnedOpacity').addEventListener('input', (e) => pinned.style.opacity = e.target.value / 100);
  $('pinnedClose').addEventListener('click', () => pinned.classList.add('hidden'));
  pinned.addEventListener('dblclick', () => {
    const next = prompt('双击展开编辑：', $('pinnedContent').textContent);
    if (next) { $('pinnedContent').textContent = next; toast('已同步回数据库 ✔'); }
  });
  // 演示：归档时自动展示置顶浮窗
  const origArchive = archiveNote;

  // ── 演示控制条 ────────────────────────────────
  $('demoTogglePanel').addEventListener('click', () => panelVisible ? hidePanel() : showPanel());
  $('demoReminder').addEventListener('click', () => showReminder('记得给产品文档补充截图'));
  $('demoReset').addEventListener('click', () => location.reload());

  // ── 引导层 ────────────────────────────────────
  $('onboardDismiss').addEventListener('click', () => {
    gsap.to($('onboarding'), { opacity: 0, scale: .95, duration: .25, onComplete: () => $('onboarding').classList.add('hidden') });
    setTimeout(showPanel, 400);
  });

  // 时钟
  setInterval(() => { $('clock').textContent = new Date().toTimeString().slice(0,5); }, 1000);

  // 初始渲染
  renderClips(); renderTodos(); renderPanelTags();
})();
