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
    // 思维导图笔记演示：type='mindmap'，卡片标题取中心节点文本，mindmapData 为 simple-mind-map 节点树
    { id: 8, type: 'mindmap', text: 'Inkling 产品规划', tags: ['产品'], date: todayStr(), time: '今天 10:05', pinned: false,
      mindmapData: { data: { text: 'Inkling 产品规划' }, children: [
        { data: { text: '核心交互' }, children: [{ data: { text: '1 秒原则' }, children: [] }, { data: { text: '顶部感应区' }, children: [] }, { data: { text: '全局快捷键' }, children: [] }] },
        { data: { text: '三态面板' }, children: [{ data: { text: '🔴 笔记' }, children: [] }, { data: { text: '🟡 粘贴板' }, children: [] }, { data: { text: '🟢 待办' }, children: [] }] },
        { data: { text: '历史归档' }, children: [{ data: { text: '热力图' }, children: [] }, { data: { text: '统计报表' }, children: [] }] },
      ] } },
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
    // 全部弹窗关闭后，若鼠标不在面板内，重新按用户配置的延迟策略排期收起
    if (modalDepth === 0 && panelVisible && !panel.matches(':hover')) {
      scheduleHide();
    }
  }
  function showPanel() {
    if (panelVisible) return;
    // 插件化配置：呼出面板被全局停用时静默拦截
    const pPanel = PLUGINS.find(p => p.id === 'panel');
    if (pPanel && !pPanel.enabled) return;
    panelVisible = true;
    panel.classList.remove('hidden');
    $('dynamicIsland').classList.add('di-fade');   // 面板展开期间灵动岛淡出（避免遮挡）
    gsap.fromTo(panel,
      { y: -30, opacity: 0, scale: .97 },
      { y: 0, opacity: 1, scale: 1, duration: .28, ease: 'back.out(1.6)' }); // 200ms 弹性滑入
    renderPanelTags();   // 呼出时刷新右下角标签区（草稿标签或"无标签"）
    updateZenToggle();   // 编辑已有笔记时展示 Zen 专注模式入口
    setTimeout(() => editor.focus(), 100);
  }
  function hidePanel() {
    if (!panelVisible) return;
    if (modalDepth > 0) return;           // 有编辑弹窗打开时禁止收起（内容需回显）
    panelVisible = false;
    setZenMode(false);                    // 收起面板时同步退出 Zen 专注模式
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
    $('dynamicIsland').classList.remove('di-fade');   // 面板收起后灵动岛恢复
  }

  // hotzone 悬停 100ms 防抖展开
  let hoverTimer;
  hotzone.addEventListener('mouseenter', () => { hoverTimer = setTimeout(showPanel, 100); });
  hotzone.addEventListener('mouseleave', () => clearTimeout(hoverTimer));

  // ── 灵动岛（Windows 顶部中央胶囊）：滚动播放待办 · 悬停展开面板 · 拖拽调节大小 ──
  // 尺寸约束：默认 320×36；宽 200-480 / 高 32-56（min/max 钳制，防止过度放大缩小）
  // 播放配置：停留时间默认 3s（1-10 可设）；范围默认仅当天待办（可切换全部未完成）
  const DI = { w: 320, h: 36, W_MIN: 200, W_MAX: 480, H_MIN: 32, H_MAX: 56, stay: 3, scope: 'today' };
  const island = $('dynamicIsland'), diTrack = $('diTrack');

  // 逐条停留轮播：每条静止展示 stay 秒 → 0.45s 平滑切换下一条；一轮播完无缝回环继续下一轮
  let diTimer = null, diIndex = 0, diCount = 0;
  function stopIslandTicker() { clearTimeout(diTimer); diTimer = null; }
  function diStep() {
    if (!diCount) return;
    diIndex++;
    diTrack.style.transition = 'transform .45s ease';
    diTrack.style.transform = `translateY(${-diIndex * DI.h}px)`;
    if (diIndex === diCount) {   // 已滚到末尾的「第一条副本」：过渡结束后无缝复位到真正第一条，完成一轮回环
      setTimeout(() => {
        diTrack.style.transition = 'none';
        diTrack.style.transform = 'translateY(0)';
        diIndex = 0;
      }, 480);
    }
    diTimer = setTimeout(diStep, DI.stay * 1000);
  }

  function renderIsland() {
    if (!diTrack) return;
    const today = todayStr();
    // 收集未完成待办（含子任务并标注归属）：默认仅当天；逾期优先，再按完成时间排序
    const pending = [];
    const inScope = (date) => DI.scope === 'all' || date === today;
    todos.forEach(t => {
      if (inScope(t.date) && !t.done) pending.push({ text: t.text, date: t.date, dueTime: t.dueTime, prio: t.priority, overdue: isOverdue(t, today) });
      t.children.forEach(c => {
        if (inScope(c.date) && !c.done) pending.push({ text: `${t.text} / ${c.text}`, date: c.date, dueTime: c.dueTime, prio: c.priority, overdue: isOverdue(c, today) });
      });
    });
    pending.sort((a, b) => (b.overdue - a.overdue) || String(a.dueTime || '').localeCompare(String(b.dueTime || '')));

    if (!pending.length) {
      stopIslandTicker(); diCount = 0;
      diTrack.style.transition = 'none';
      diTrack.style.transform = 'translateY(0)';
      diTrack.innerHTML = `<div class="di-item" style="height:${DI.h}px"><span class="di-dot idle"></span><span class="di-text dim">${DI.scope === 'today' ? '今日待办已全部完成 🎉' : '没有未完成待办 🎉'}</span></div>`;
      return;
    }
    const items = pending.map(t => `
      <div class="di-item" style="height:${DI.h}px">
        <span class="di-dot ${t.prio}"></span>
        <span class="di-text">${t.date !== today ? `<span class="di-date">${t.date.slice(5)}</span>` : ''}${escapeHtml(t.text)}</span>
        <span class="di-time ${t.overdue ? 'ovd' : ''}">${t.overdue ? '逾期 ' + (t.dueTime || '') : (t.dueTime || '')}</span>
      </div>`);
    // 逐条停留轮播：末尾复制第一条，滚到副本后无缝复位；仅一条时静止展示
    stopIslandTicker();
    diIndex = 0;
    diCount = pending.length;
    diTrack.innerHTML = items.join('') + (pending.length > 1 ? items[0] : '');
    diTrack.style.transition = 'none';
    diTrack.style.transform = 'translateY(0)';
    if (pending.length > 1) diTimer = setTimeout(diStep, DI.stay * 1000);
  }

  // 悬停 100ms 防抖展开主面板（与 hotzone 同一交互）；悬停时暂停轮播，移开后恢复
  let diHoverTimer;
  island.addEventListener('mouseenter', () => { diHoverTimer = setTimeout(showPanel, 100); stopIslandTicker(); });
  island.addEventListener('mouseleave', () => {
    clearTimeout(diHoverTimer);
    if (diCount > 1 && !diTimer) diTimer = setTimeout(diStep, DI.stay * 1000);   // 恢复轮播
  });

  // 右下角手柄拖拽调节：宽 200-480（水平居中对称伸缩）/ 高 32-56，min/max 钳制
  let diDrag = null;
  $('diResizer').addEventListener('mousedown', (e) => {
    diDrag = { x: e.clientX, y: e.clientY, w: DI.w, h: DI.h };
    e.preventDefault();
  });
  document.addEventListener('mousemove', (e) => {
    if (!diDrag) return;
    DI.w = Math.min(DI.W_MAX, Math.max(DI.W_MIN, Math.round(diDrag.w + (e.clientX - diDrag.x) * 2)));
    DI.h = Math.min(DI.H_MAX, Math.max(DI.H_MIN, Math.round(diDrag.h + (e.clientY - diDrag.y))));
    island.style.width = DI.w + 'px';
    island.style.height = DI.h + 'px';
  });
  document.addEventListener('mouseup', () => { if (diDrag) { diDrag = null; renderIsland(); } });

  // ══ 偏好设置持久化 ══
  // 审查发现：原型除主题外的设置项刷新即丢失，且「失焦策略 / 粘贴板保留天数 /
  // 开机静默自启动」三项完全没有读写逻辑（见需求 6.4）。此处统一收口——每一项
  // 都有默认值、持久化与启动恢复，杜绝「只有 UI、没有行为」的装饰性开关（需求 2.7）。
  const PREF_KEY = 'inkling-prefs';
  const PREF_DEFAULTS = {
    blur: '延迟 3 秒收起', remarkStyle: 'auto',
    island: true, islandStay: 3, islandScope: 'today',
    islandOpacity: 100, islandGlow: false, islandAutoHide: true,
    clipRetention: 30, autoStart: true,
  };
  let prefs = { ...PREF_DEFAULTS };
  function loadPrefs() {
    try {
      prefs = { ...PREF_DEFAULTS, ...JSON.parse(localStorage.getItem(PREF_KEY) || '{}') };
    } catch {
      prefs = { ...PREF_DEFAULTS };   // 存储损坏时回退默认值，不阻塞启动
    }
  }
  function setPref(key, value) {
    prefs = { ...prefs, [key]: value };
    try { localStorage.setItem(PREF_KEY, JSON.stringify(prefs)); }
    catch { /* 隐私模式下 localStorage 不可写：静默降级为仅本次会话生效 */ }
  }
  // 失焦收起延迟：由「立即收起 / 延迟 3 秒收起 / 固定不收起」三档策略决定（需求 2.1）
  function blurDelay() {
    if (prefs.blur === '立即收起') return 0;
    if (prefs.blur === '固定不收起') return null;   // null = 不排期收起
    return 3000;
  }
  function scheduleHide() {
    const delay = blurDelay();
    clearTimeout(hideTimer);
    if (delay === null) return;
    hideTimer = setTimeout(hidePanel, delay);
  }
  // 启动时把已保存的偏好回填到控件，并应用各自的副作用
  function applyPrefs() {
    loadPrefs();
    $('settingBlur').value = prefs.blur;
    $('settingRemarkStyle').value = prefs.remarkStyle;
    $('settingIsland').checked = prefs.island;
    $('settingIslandStay').value = prefs.islandStay;
    $('settingIslandScope').value = prefs.islandScope;
    $('settingIslandOpacity').value = prefs.islandOpacity;
    $('settingIslandGlow').checked = prefs.islandGlow;
    $('settingIslandAutoHide').checked = prefs.islandAutoHide;
    $('settingClipRetention').value = prefs.clipRetention;
    $('settingAutoStart').checked = prefs.autoStart;
    DI.stay = prefs.islandStay;
    DI.scope = prefs.islandScope;
    island.classList.toggle('hidden', !prefs.island);
    island.classList.toggle('di-glow', prefs.islandGlow);
    island.style.opacity = prefs.islandOpacity / 100;
  }

  // 设置页开关：关闭时隐藏胶囊
  $('settingIsland').addEventListener('change', (e) => {
    island.classList.toggle('hidden', !e.target.checked);
    setPref('island', e.target.checked);
    toast(e.target.checked ? '灵动岛已开启' : '灵动岛已关闭');
  });
  // 设置页：滚动停留时间（秒，1-10 钳制）与播放范围（仅当天 / 全部未完成），即时生效
  $('settingIslandStay').addEventListener('change', (e) => {
    DI.stay = Math.min(10, Math.max(1, Number(e.target.value) || 3));
    e.target.value = DI.stay;
    setPref('islandStay', DI.stay);
    renderIsland();
    toast(`灵动岛停留时间已设为 ${DI.stay} 秒`);
  });
  $('settingIslandScope').addEventListener('change', (e) => {
    DI.scope = e.target.value;
    setPref('islandScope', DI.scope);
    renderIsland();
    toast(DI.scope === 'today' ? '灵动岛仅播放当天待办' : '灵动岛播放全部未完成待办');
  });
  // 设置页：透明度（30%-100%，参考置顶浮窗）与流光边框开关（参考 NetSpeed-Dynamic），即时生效
  $('settingIslandOpacity').addEventListener('input', (e) => {
    island.style.opacity = e.target.value / 100;
    setPref('islandOpacity', Number(e.target.value));
  });
  $('settingIslandGlow').addEventListener('change', (e) => {
    island.classList.toggle('di-glow', e.target.checked);
    setPref('islandGlow', e.target.checked);
    toast(e.target.checked ? '流光边框已开启 ✨' : '流光边框已关闭');
  });
  // 以下四项此前为纯装饰控件（无 id、无逻辑），本轮补齐读写与持久化
  $('settingBlur').addEventListener('change', (e) => {
    setPref('blur', e.target.value);
    toast(`失焦策略已设为「${e.target.value}」`);
  });
  $('settingIslandAutoHide').addEventListener('change', (e) => {
    setPref('islandAutoHide', e.target.checked);
    // 原型无法真实检测其他应用全屏，此处仅持久化配置（见需求 2.8 / 3.5）
    toast(e.target.checked ? '全屏时将自动隐藏灵动岛' : '全屏时保持显示灵动岛');
  });
  $('settingClipRetention').addEventListener('change', (e) => {
    const days = Math.min(365, Math.max(1, Number(e.target.value) || 30));
    e.target.value = days;
    setPref('clipRetention', days);
    toast(`粘贴板保留天数已设为 ${days} 天`);
  });
  $('settingAutoStart').addEventListener('change', (e) => {
    setPref('autoStart', e.target.checked);
    toast(e.target.checked ? '已开启开机静默自启动' : '已关闭开机自启动');
  });
  $('settingHotkeyRecord').addEventListener('click', () => {
    // 真实实现须校验系统与应用内占用冲突，并支持注册失败降级（见需求 2.1）
    toast('原型不支持录制全局快捷键（需系统级热键注册能力）');
  });

  // 提醒事件岛内呈现（参考 NetSpeed-Dynamic）：提醒到达时岛屿切换为提醒卡片（轮播暂停），
  // 6 秒后或点击后恢复轮播；与右上角提醒浮窗并存
  let diAlertTimer = null;
  function showIslandAlert(content) {
    if (island.classList.contains('hidden')) return;   // 灵动岛关闭时不呈现
    clearTimeout(diAlertTimer);
    stopIslandTicker();
    $('diAlert').innerHTML = `<span class="di-alert-ico">⏰</span><span class="di-alert-text">提醒：${escapeHtml(content)}</span>`;
    $('diAlert').classList.remove('hidden');
    diAlertTimer = setTimeout(hideIslandAlert, 6000);
  }
  function hideIslandAlert() {
    clearTimeout(diAlertTimer); diAlertTimer = null;
    if ($('diAlert').classList.contains('hidden')) return;
    $('diAlert').classList.add('hidden');
    if (diCount > 1 && !diTimer) diTimer = setTimeout(diStep, DI.stay * 1000);   // 恢复轮播
  }
  $('diAlert').addEventListener('click', hideIslandAlert);

  renderIsland();   // 初始化滚动播放
  // 面板失焦 → 按设置项「立即收起 / 延迟 3 秒收起 / 固定不收起」三档策略处理
  panel.addEventListener('mouseleave', () => {
    if (modalDepth > 0) return;           // 弹窗打开时：不因失焦收起
    scheduleHide();
  });
  panel.addEventListener('mouseenter', () => clearTimeout(hideTimer));

  // 键盘：Esc 收起 / ⌃1-3 切态 / ⌃⇧Space 呼出
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      // Zen 专注模式 / 启动台 / 导图通用弹窗 / 导图搜索框 / 导图抽屉 / 导图全屏 / 思维导图独立窗口 / 标签管理弹窗 / 编辑浮框打开时优先关闭
      if (panel.classList.contains('zen-mode')) { setZenMode(false); return; }
      if (!$('launcherWindow').classList.contains('hidden')) { closeLauncher(); return; }
      if (!$('mmModalOverlay').classList.contains('hidden')) { closeMmModal(); return; }
      if (!$('mmSearchBox').classList.contains('hidden')) { closeMindmapSearch(); return; }
      if (!$('mindmapCtxMenu').classList.contains('hidden')) { mmHideCtx(); return; }   // 先关导图右键菜单
      if (!$('mmDrawer').classList.contains('hidden')) {
        $('mmDrawer').classList.add('hidden');
        document.querySelectorAll('.mm-dock-item').forEach(x => x.classList.remove('active'));
        return;
      }
      if ($('mindmapWindow').classList.contains('mm-fullscreen')) {
        $('mindmapWindow').classList.remove('mm-fullscreen');
        $('mmToggleFullscreen').style.color = '';
        setTimeout(() => { if (mindMap) { mindMap.resize(); mindMap.view.fit(); mindmapZoomText(); } }, 60);
        return;
      }
      // 节点文本编辑中按 Esc 仅退出节点编辑框（交由导图库处理），不关闭整个编辑窗口
      if (!$('mindmapWindow').classList.contains('hidden')) {
        const editingNode = mindMap && mindMap.renderer && mindMap.renderer.textEdit && mindMap.renderer.textEdit.isShowTextEdit();
        if (!editingNode) closeMindmapEditor();
        return;
      }
      if (!$('tagManagerOverlay').classList.contains('hidden')) { closeTagManager(); return; }
      if (!$('clipEditorOverlay').classList.contains('hidden')) { closeClipEditor(); return; }
      if (!$('todoEditorOverlay').classList.contains('hidden')) { closeTodoEditor(); return; }
      if (!$('prioMenu').classList.contains('hidden')) { hidePrioMenu(); return; }
      if (!$('repeatMenu').classList.contains('hidden')) { hideRepeatMenu(); return; }
      hidePanel(); closeAllWindows(); $('trayMenu').classList.add('hidden');
    }
    // 快捷键呼出启动台：Alt+Space 或 ⌥Space（参考 Wox / ZeroLaunch-rs）
    if (e.altKey && e.code === 'Space') {
      e.preventDefault();
      $('launcherWindow').classList.contains('hidden') ? openLauncher() : closeLauncher();
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
    document.querySelectorAll('.nav-dot').forEach(d => {
      const on = d.dataset.mode === mode;
      d.classList.toggle('active', on);
      d.setAttribute('aria-selected', String(on));   // 无障碍：与视觉激活态同步（需求 2.2）
    });
    ['note','clipboard','todo'].forEach(m => $('page-' + m).classList.toggle('hidden', m !== mode));
    if (mode === 'clipboard') renderClips();
    if (mode === 'todo') renderTodos();
    updateZenToggle();
  }
  document.querySelectorAll('.nav-dot').forEach(d => d.addEventListener('click', () => switchMode(d.dataset.mode)));

  // ── Zen 专注模式（仅编辑已有笔记时入口可见；面板全屏化沉浸写作） ──
  function setZenMode(on) {
    panel.classList.toggle('zen-mode', on);
    if (on) setTimeout(() => editor.focus(), 60);
    updateZenToggle();
  }
  // Zen 入口展示条件：面板展开 + 笔记模式 + 编辑已有笔记（从主窗口/启动台打开）+ 未处于 Zen 中
  function updateZenToggle() {
    const inNoteMode = !$('page-note').classList.contains('hidden');
    const show = panelVisible && inNoteMode && editingNoteId !== null && !panel.classList.contains('zen-mode');
    $('zenToggle').classList.toggle('hidden', !show);
  }
  $('zenToggle').addEventListener('click', () => setZenMode(true));
  $('zenExit').addEventListener('click', () => setZenMode(false));

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

  // ── 🟡 粘贴板：写入新条目 ──
  // 统一入口：思维导图「复制大纲」、启动台计算结果等场景写回剪贴板均走此处，
  // 保证新条目同时反映到面板列表、归档列表与当月热力图。
  function addClip(text, type = 'text') {
    const content = String(text ?? '').trim();
    if (!content) { toast('内容为空，未写入剪贴板'); return null; }
    const clip = { id: Date.now(), type, text: content, date: todayStr(), time: nowTimeStr(), pinned: false };
    clips.unshift(clip);
    renderClips($('clipSearch')?.value || '');   // 保持当前搜索过滤态
    renderArchive();                             // 同步归档列表与当月热力图
    return clip;
  }

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
  // q 非空 + todayOnly=false：归档跨日期搜索模式（含子任务文本，结果显示所属日期）
  // prio：优先级过滤（'all' = 全部）；todayOnly=true：仅当日视图范围（面板），q 在当日范围内过滤
  function renderTodoList(el, viewDate, q = '', prio = 'all', todayOnly = false) {
    const pool = todos.filter(t => prio === 'all' || t.priority === prio);
    if (q && q.trim() && !todayOnly) {
      const needle = q.trim().toLowerCase();
      const match = (t) => t.text.toLowerCase().includes(needle);
      const hits = pool.filter(t => match(t) || t.children.some(match))
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
    const needle = (q || '').trim().toLowerCase();
    const hit = (t) => !needle || t.text.toLowerCase().includes(needle) || t.children.some(c => c.text.toLowerCase().includes(needle));
    const pulled = pool.filter(t =>
      hit(t) && (isOverdue(t, viewDate) || t.children.some(c => isOverdue(c, viewDate))));
    const pulledIds = new Set(pulled.map(t => t.id));
    const normal = pool.filter(t => t.date === viewDate && !pulledIds.has(t.id) && hit(t));
    const pulledHtml = pulled.length
      ? `<li class="todo-section">⚠️ 逾期事项 · 按完成时间与优先级置顶（${pulled.length} 项）</li>` +
        sortOverdue(pulled).map(t => todoItemHTML(t, viewDate)).join('')
      : '';
    const normalHtml = normal.length ? sortTodos(normal).map(t => todoItemHTML(t, viewDate)).join('') : '';
    const emptyMsg = needle ? `未找到匹配「${escapeHtml(q.trim())}」的当日待办` : `该日暂无${prio === 'all' ? '' : PRIORITY[prio].label + '优先级'}待办事项`;
    el.innerHTML = (pulledHtml + normalHtml) || `<div class="todo-empty">${emptyMsg}</div>`;
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
    const opt = (p) => `<div class="prio-opt ${p}" data-prio="${p}"><span class="prio-dot"></span>${PRIORITY[p].label}</div>`;
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

  // 面板：当日视图 + 搜索/优先级过滤（归档若开着则带搜索词同步刷新）；数据变化同步迷你热力图与日期详情
  function renderTodos() {
    renderTodoList($('todoList'), todayStr(), $('todoPanelSearch').value, $('todoPrioFilter').value, true);
    const arch = document.getElementById('archiveTodoList');
    if (arch) renderTodoList(arch, archiveViewDate, ($('todoArchiveSearch').value || '').trim());
    renderMiniHeat();
    renderDayDetail();
    renderIsland();   // 待办变化同步灵动岛滚动播放
  }
  let archiveViewDate = todayStr();   // 归档页当前查看日期
  bindTodoList($('todoList'), () => todayStr());           // 面板事件委托（锁死当天）
  bindTodoList($('archive-todos'), () => archiveViewDate); // 归档事件委托（委托到外部容器，内部列表动态渲染）

  // 面板待办：搜索框与优先级过滤器
  $('todoPanelSearch').addEventListener('input', renderTodos);
  $('todoPrioFilter').addEventListener('change', renderTodos);
  $('todoDueBtn').addEventListener('click', () => openTodoEditor({ mode: 'create' }));   // 新增待办（弹窗内设置完成时间）

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
    showIslandAlert(content);   // 灵动岛同步呈现提醒卡片（参考 NetSpeed-Dynamic 事件岛内呈现）
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
    document.querySelectorAll('.side-item').forEach(x => {
      const on = x.dataset.view === view;
      x.classList.toggle('active', on);
      x.setAttribute('aria-selected', String(on));   // 无障碍：与视觉激活态同步
    });
    $('sideSettings').classList.toggle('active', view === 'settings');
    $('sideStats').classList.toggle('active', view === 'stats');
    ['notes','clips','todos','settings','stats','day','plugins'].forEach(v => {
      const el = $('archive-' + v);
      if (el) el.classList.toggle('hidden', v !== view);
    });
    if (view === 'notes' || view === 'clips') renderArchive();
    if (view === 'todos') renderTodos();
    if (view === 'stats') renderStats();
    if (view === 'day') renderDayDetail();
    if (view === 'plugins') renderPlugins();
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
  $('settingRemarkStyle').addEventListener('change', (e) => {
    setPref('remarkStyle', e.target.value);
    renderTodos();
  });

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

    // 笔记：置顶优先 + 模糊搜索（正文、标签，思维导图额外匹配全部节点文本）
    const sortedNotes = [...notes]
      .sort((a, b) => (b.pinned - a.pinned))
      .filter(n => !noteFilter ||
        n.text.toLowerCase().includes(noteFilter) ||
        (n.tags || []).some(t => t.toLowerCase().includes(noteFilter)) ||
        (n.type === 'mindmap' && mindmapAllText(n.mindmapData).includes(noteFilter)));
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
        <div class="a-text">${n.type === 'mindmap' ? '<span class="clip-type mindmap">🧠 思维导图</span>' : ''}${renderMdCard(n.text)}</div>
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
  // 回显笔记到主面板编辑（归档列表 ✏️ / 日期详情 / 启动台共用）：
  // 关闭历史窗 → 展开主面板 → 内容填入编辑器进入编辑模式（右下角回显该笔记标签，Zen 入口随编辑态显示）
  function openNoteInPanel(id) {
    const n = notes.find(x => x.id === id);
    if (!n) return;
    editingNoteId = id;
    closeAllWindows();
    switchMode('note');
    showPanel();
    editor.innerText = n.text;
    renderInlineMarkdown(); renderPanelTags();
    $('btnArchive').textContent = '保存修改 ✓';
    toast('内容已回显，修改后点击「保存修改」');
  }

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
      // 思维导图笔记：弹出独立编辑窗口继续编辑（不回显主面板）
      if (n.type === 'mindmap') { openMindmapEditor(id); return; }
      openNoteInPanel(id);   // 回显主面板进入编辑模式（Zen 专注模式入口随编辑态显示）
    }
  });

  // ── 思维导图编辑器（独立窗口，参考 wanglin2/mind-map，simple-mind-map UMD 版） ──
  // 闭环：笔记列表「🧠 思维导图」→ 弹出独立编辑窗口（与主窗口并存，主窗口隐藏/关闭互不影响）
  //      → 保存为 type='mindmap' 笔记卡片 → ✏️ 再次编辑回显；画布支持滚轮缩放 + 左键拖动平移
  let mindMap = null;              // simple-mind-map 实例（惰性初始化，复用同一画布）
  let editingMindmapId = null;     // null = 新建；否则为正在编辑的笔记 id
  const MINDMAP_DEFAULT = () => ({
    data: {
      text: '减脂计划',
      image: 'https://images.unsplash.com/photo-1490645935967-10de6ba17061?w=320&auto=format&fit=crop&q=80',
      imageTitle: '健康饮食',
      imageSize: { width: 140, height: 95 }
    },
    children: [
      {
        data: { text: '1. 饮食调整' },
        children: [
          { data: { text: '⭐ 控制每日热量摄入，计算基础代谢率并适当减少热量' }, children: [] },
          { data: { text: '🥗 增加蔬菜、水果摄入，保证每餐蔬菜占比至少一半' }, children: [] },
          { data: { text: '🍗 选择优质蛋白质，如鸡胸肉、鱼虾、豆类等' }, children: [] }
        ]
      },
      {
        data: { text: '2. 运动安排' },
        children: [
          { data: { text: '🏃 慢跑，每周3-4次，每次30分钟以上' }, children: [] },
          { data: { text: '🏊 游泳，每周2-3次，每次持续40分钟左右' }, children: [] },
          { data: { text: '🏋 力量训练，每周2-3次' }, children: [] }
        ]
      },
      {
        data: { text: '3. 生活习惯养成' },
        children: [
          { data: { text: '😴 保证充足睡眠，每天7-8小时' }, children: [] },
          { data: { text: '🧘 减少压力，通过冥想、瑜伽等方式放松身心' }, children: [] }
        ]
      }
    ]
  });
  // 剥离富文本 HTML 取纯文本（simple-mind-map 默认富文本模式，节点 text 带 <p> 等标签）
  function stripRichText(html) {
    const d = document.createElement('div');
    d.innerHTML = String(html || '');
    return d.textContent || '';
  }
  // 递归收集导图全部节点文本（供列表搜索匹配，返回小写串）
  function mindmapAllText(node) {
    if (!node) return '';
    const self = stripRichText(node.data && node.data.text);
    return (self + ' ' + (node.children || []).map(mindmapAllText).join(' ')).toLowerCase();
  }
  // 统计导图字数与节点数（对齐 wanglin2/mind-map 左下角统计）
  function countMindmapStats(node) {
    if (!node) return { chars: 0, nodes: 0 };
    const selfTxt = stripRichText(node.data && node.data.text);
    let chars = selfTxt.replace(/\s+/g, '').length;
    let nodes = 1;
    (node.children || []).forEach(c => {
      const sub = countMindmapStats(c);
      chars += sub.chars;
      nodes += sub.nodes;
    });
    return { chars, nodes };
  }
  function updateMindmapStats() {
    if (!mindMap) return;
    const tree = mindMap.getData(false);
    const stats = countMindmapStats(tree);
    if ($('mmStatLeft')) $('mmStatLeft').textContent = `字数 ${stats.chars}   节点 ${stats.nodes}`;
  }
  // 同步工具栏缩放比例显示
  function mindmapZoomText() {
    if (mindMap) $('mindmapZoomLabel').textContent = Math.round(mindMap.view.scale * 100) + ' %';
  }
  function openMindmapEditor(noteId) {
    const Ctor = window.simpleMindMap && (window.simpleMindMap.default || window.simpleMindMap);
    if (!Ctor) { toast('思维导图组件加载失败（CDN 未就绪），请刷新重试'); return; }
    editingMindmapId = noteId;
    $('mindmapBarTitle').textContent = noteId ? '编辑思维导图' : '新建思维导图';
    const data = noteId
      ? JSON.parse(JSON.stringify(notes.find(x => x.id === noteId).mindmapData))   // 深拷贝，避免直接改到列表数据
      : MINDMAP_DEFAULT();
    const rootTitle = stripRichText(data && data.data && data.data.text).trim() || '未命名导图';
    updateFilenameDisplay(rootTitle);
    // 独立窗口：仅显隐本窗口，不切换主窗口视图；主窗口隐藏/关闭互不影响
    $('mindmapWindow').classList.remove('hidden');
    if (!mindMap) {
      mindMap = new Ctor({
        el: $('mindmapContainer'), data, readonly: false,
        mousewheelAction: 'zoom',          // 滚轮缩放（库默认为 move 上下平移）；左键拖动平移为库默认行为
      });
      // 节点右键：激活该节点并弹出上下文菜单（参考 wanglin2/mind-map）
      mindMap.on('node_contextmenu', (e, node) => {
        e.preventDefault();
        mindMap.renderer.clearActiveNodeList();
        mindMap.renderer.addNodeToActiveList(node);
        mmShowCtx(e.clientX, e.clientY);
      });
      // 节点树变化时更新左下角统计（字数 / 节点数）
      mindMap.on('node_tree_render_end', updateMindmapStats);
      mindMap.on('data_change', updateMindmapStats);
    } else {
      mindMap.setData(data);
      // 打开已保存导图时重置视图：定位到中心主题，防止上次平移/缩放导致内容出界找不到
      mindMap.view.fit();
      mindMap.view.reset();
    }
    mindmapZoomText();
    updateMindmapStats();
  }
  function closeMindmapEditor() {
    $('mindmapWindow').classList.add('hidden');
    mmHideCtx();
    editingMindmapId = null;
  }
  function saveMindmap() {
    if (!mindMap) { closeMindmapEditor(); return; }
    const data = mindMap.getData(false);   // 纯节点树 { data, children }
    const rootText = stripRichText(data && data.data && data.data.text).trim() || '未命名导图';
    if (editingMindmapId) {
      const n = notes.find(x => x.id === editingMindmapId);
      if (n) { n.text = rootText; n.mindmapData = data; n.date = todayStr(); n.time = '刚刚'; }
      toast('思维导图已更新 🧠');
    } else {
      notes.unshift({ id: Date.now(), type: 'mindmap', text: rootText, tags: [], date: todayStr(), time: '刚刚', pinned: false, mindmapData: data });
      toast('思维导图已保存到笔记 🧠');
    }
    closeMindmapEditor();
    renderArchive();   // 主窗口笔记列表即时刷新（窗口隐藏时刷新不可见，但数据保持一致）
  }
  $('noteArchNewMindmap').addEventListener('click', () => openMindmapEditor(null));
  $('mmSave').addEventListener('click', saveMindmap);
  $('mmCancel').addEventListener('click', closeMindmapEditor);
  $('mmNew').addEventListener('click', () => {
    if (confirm('新建导图将清空当前未保存的内容，确认新建？')) {
      editingMindmapId = null;
      $('mindmapBarTitle').textContent = '新建思维导图';
      mindMap && mindMap.setData(MINDMAP_DEFAULT());
      mindMap && mindMap.view.fit();
      mindmapZoomText(); updateMindmapStats();
    }
  });
  // 画布缩放控制：底部控制栏 ＋/−/适应 + 滚轮（构造时已设 mousewheelAction: 'zoom'），左键拖动平移为库默认行为
  $('mindmapZoomIn').addEventListener('click', () => { if (mindMap) { mindMap.view.enlarge(); mindmapZoomText(); } });
  $('mindmapZoomOut').addEventListener('click', () => { if (mindMap) { mindMap.view.narrow(); mindmapZoomText(); } });
  $('mindmapFit').addEventListener('click', () => { if (mindMap) { mindMap.view.fit(); mindmapZoomText(); } });
  $('mindmapContainer').addEventListener('wheel', () => setTimeout(mindmapZoomText, 0), { passive: true });
  // 底部定位、只读切换、小地图与全屏
  $('mmLocateCenter').addEventListener('click', () => { if (mindMap) { mindMap.view.fit(); mindmapZoomText(); } });
  let mmIsReadonly = false;
  $('mmToggleReadonly').addEventListener('click', () => {
    if (!mindMap) return;
    mmIsReadonly = !mmIsReadonly;
    mindMap.setReadonly(mmIsReadonly);
    $('mmToggleReadonly').style.color = mmIsReadonly ? '#1677ff' : '';
    toast(mmIsReadonly ? '已切换为只读浏览模式 🧭' : '已切换为编辑模式 ✏️');
  });

  // 小地图导航开关与简易渲染
  function renderMinimap() {
    const box = $('mmMinimap');
    if (box.classList.contains('hidden') || !mindMap) return;
    const cvs = $('mmMinimapCanvas');
    const ctx = cvs.getContext('2d');
    cvs.width = box.clientWidth; cvs.height = box.clientHeight;
    ctx.clearRect(0, 0, cvs.width, cvs.height);
    // 简易绘制当前导图结构缩略图
    ctx.fillStyle = '#1677ff';
    ctx.beginPath();
    ctx.roundRect(14, cvs.height / 2 - 10, 42, 20, 4);
    ctx.fill();
    ctx.fillStyle = '#52c41a';
    ctx.beginPath();
    ctx.roundRect(76, cvs.height / 2 - 28, 46, 16, 3);
    ctx.roundRect(76, cvs.height / 2 + 12, 46, 16, 3);
    ctx.fill();
    ctx.strokeStyle = '#ccc'; ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(56, cvs.height / 2); ctx.lineTo(76, cvs.height / 2 - 20);
    ctx.moveTo(56, cvs.height / 2); ctx.lineTo(76, cvs.height / 2 + 20);
    ctx.stroke();
  }
  $('mmToggleMinimap').addEventListener('click', () => {
    const box = $('mmMinimap');
    box.classList.toggle('hidden');
    if (!box.classList.contains('hidden')) { renderMinimap(); toast('已开启小地图 🗺'); }
    else toast('已关闭小地图');
  });
  $('mmMinimapClose').addEventListener('click', () => $('mmMinimap').classList.add('hidden'));

  // 全屏编辑/查看切换
  $('mmToggleFullscreen').addEventListener('click', () => {
    const win = $('mindmapWindow');
    const isFull = win.classList.toggle('mm-fullscreen');
    $('mmToggleFullscreen').style.color = isFull ? '#1677ff' : '';
    setTimeout(() => { if (mindMap) { mindMap.resize(); mindMap.view.fit(); mindmapZoomText(); } }, 60);
    toast(isFull ? '已进入导图全屏模式 ⛶（点击按钮或 Esc 退出）' : '已退出全屏');
  });
  // ── 搜索框浮层（对齐 Image #13：右上角悬浮药丸型搜索框） ─────
  function openMindmapSearch() {
    const box = $('mmSearchBox');
    box.classList.remove('hidden');
    $('mmSearchInput').value = '';
    setTimeout(() => $('mmSearchInput').focus(), 40);
  }
  function closeMindmapSearch() { $('mmSearchBox').classList.add('hidden'); }
  $('mmSearchNode').addEventListener('click', openMindmapSearch);
  $('mmSearchClose').addEventListener('click', closeMindmapSearch);
  $('mmSearchInput').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const q = e.target.value.trim();
      if (!q) return;
      if (mindMap) {
        const match = mindmapAllText(mindMap.getData(false)).includes(q.toLowerCase());
        toast(match ? `已定位到包含「${q}」的节点 🎯` : `未在导图中找到「${q}」`);
      }
    }
    if (e.key === 'Escape') closeMindmapSearch();
  });

  // ── 通用节点弹窗系统（对齐 Image #14-#20：备注/标签/图标/图片/公式/超链接/外框） ──
  let mmModalCallback = null;
  function openMmModal(title, bodyHtml, onConfirm) {
    $('mmModalTitle').textContent = title;
    $('mmModalBody').innerHTML = bodyHtml;
    $('mmModalOverlay').classList.remove('hidden');
    mmModalCallback = onConfirm;
  }
  function closeMmModal() {
    $('mmModalOverlay').classList.add('hidden');
    mmModalCallback = null;
  }
  $('mmModalClose').addEventListener('click', closeMmModal);
  $('mmModalCancel').addEventListener('click', closeMmModal);
  $('mmModalConfirm').addEventListener('click', () => {
    if (mmModalCallback) mmModalCallback();
    closeMmModal();
  });

  // [Image #14] 备注弹窗：富文本/Markdown 顶部快捷排版工具条 + 编辑区 + 底部模式指示
  $('mmRemark').addEventListener('click', () => {
    if (mmNeedNode()) return;
    const curNote = (mmActiveNode()?.nodeData?.data?.note) || '';
    const body = `
      <div style="display:flex;gap:6px;align-items:center;padding:6px 8px;background:#f5f5f5;border-radius:6px;margin-bottom:10px;font-size:12px;color:#555;">
        <button class="icon-btn" style="font-weight:700;">H</button>
        <button class="icon-btn" style="font-weight:700;">B</button>
        <button class="icon-btn" style="font-style:italic;">I</button>
        <button class="icon-btn" style="text-decoration:line-through;">S</button>
        <span class="mm-sep"></span>
        <button class="icon-btn">”</button>
        <button class="icon-btn">≡</button>
        <button class="icon-btn">☑</button>
        <span style="margin-left:auto;font-size:11px;color:#999;">支持 Markdown 语法</span>
      </div>
      <textarea class="mm-textarea" id="mmRemarkText" placeholder="在此输入节点详细备注…">${escapeHtml(curNote)}</textarea>
      <div style="display:flex;justify-content:flex-end;gap:12px;margin-top:8px;font-size:11px;color:#999;">
        <span>Markdown</span><span style="color:#1677ff;cursor:pointer;">WYSIWYG</span>
      </div>`;
    openMmModal('备注', body, () => {
      const val = $('mmRemarkText').value.trim();
      mmExec('SET_NODE_NOTE', val);
      toast(val ? '已设置节点备注 📝' : '已清空节点备注');
    });
    setTimeout(() => $('mmRemarkText').focus(), 50);
  });

  // [Image #15] 标签弹窗：极简居中单输入框「请按回车键添加」
  $('mmTag').addEventListener('click', () => {
    if (mmNeedNode()) return;
    const curTags = (mmActiveNode()?.nodeData?.data?.tag || []).join(', ');
    const body = `
      <div class="mm-input-row">
        <input class="mm-input-text" id="mmTagInput" placeholder="请按回车键添加（多个标签用逗号隔开）" value="${escapeHtml(curTags)}">
      </div>`;
    openMmModal('标签', body, () => {
      const val = $('mmTagInput').value.trim();
      const tags = val.split(/[,，]/).map(x => x.trim()).filter(Boolean);
      mmExec('SET_NODE_TAG', tags);
      toast(tags.length ? `已设置标签：[${tags.join(', ')}] 🏷` : '已清空标签');
    });
    setTimeout(() => $('mmTagInput').focus(), 50);
  });

  // [Image #16] 图标 / 贴纸弹窗：优先级(1-10)、进度圆饼、表情、标记图标网格
  $('mmIcon').addEventListener('click', () => {
    if (mmNeedNode()) return;
    const PRIO_ICONS = ['①','②','③','④','⑤','⑥','⑦','⑧','⑨','⑩'];
    const PROG_ICONS = ['◔','◑','◕','●','✔'];
    const EMOJI_ICONS = ['😀','😍','🤔','😭','😡','🎉','🔥','💡','⭐','🚩'];
    const MARK_ICONS = ['📌','📎','🔒','🔑','🎯','🚀','⚡','⚠','❓','❤'];
    const body = `
      <div style="display:flex;gap:16px;border-bottom:1px solid #eee;padding-bottom:8px;margin-bottom:12px;font-weight:600;font-size:13px;">
        <span style="color:#1677ff;border-bottom:2px solid #1677ff;padding-bottom:6px;cursor:pointer;">图标</span>
        <span style="color:#888;cursor:pointer;">贴纸</span>
      </div>
      <div style="font-size:11.5px;font-weight:600;color:#888;margin-bottom:6px;">优先级图标</div>
      <div class="mm-icon-grid" style="margin-bottom:14px;">
        ${PRIO_ICONS.map((ico, i) => `<div class="mm-icon-tile" data-ico="priority_${i+1}">${ico}</div>`).join('')}
      </div>
      <div style="font-size:11.5px;font-weight:600;color:#888;margin-bottom:6px;">任务进度图标</div>
      <div class="mm-icon-grid" style="margin-bottom:14px;">
        ${PROG_ICONS.map((ico, i) => `<div class="mm-icon-tile" data-ico="progress_${i+1}">${ico}</div>`).join('')}
      </div>
      <div style="font-size:11.5px;font-weight:600;color:#888;margin-bottom:6px;">表情与标记</div>
      <div class="mm-icon-grid">
        ${[...EMOJI_ICONS, ...MARK_ICONS].map(ico => `<div class="mm-icon-tile" data-ico="emoji">${ico}</div>`).join('')}
      </div>`;
    openMmModal('图标/贴纸', body, null);
    document.querySelectorAll('.mm-icon-tile').forEach(tile => {
      tile.addEventListener('click', () => {
        const ico = tile.innerText.trim();
        mmExec('SET_NODE_ICON', [tile.dataset.ico]);
        closeMmModal();
        toast(`已为节点设置图标 ${ico}`);
      });
    });
  });

  // [Image #17] 图片弹窗：方式一（拖拽/点击选择）＋ 方式二（输入图片URL + 可选标题）
  $('mmImg').addEventListener('click', () => {
    if (mmNeedNode()) return;
    const body = `
      <div class="mm-input-row">
        <label>方式一</label>
        <div class="mm-upload-dropzone" id="mmImgDrop">
          <div style="font-size:24px;margin-bottom:6px;">📁</div>
          <div>点击此处选择图片、或拖动图片到此</div>
        </div>
      </div>
      <div class="mm-input-row">
        <label>方式二</label>
        <input class="mm-input-text" id="mmImgUrl" placeholder="请输入图片在线地址：http://xxx.com/xx.jpg" value="https://images.unsplash.com/photo-1579783902614-a3fb3927b675?w=300">
      </div>
      <div class="mm-input-row">
        <label>可选：图片标题</label>
        <input class="mm-input-text" id="mmImgTitle" placeholder="输入图片描述或标题">
      </div>`;
    openMmModal('插入图片', body, () => {
      const url = $('mmImgUrl').value.trim();
      const title = $('mmImgTitle').value.trim() || '图片';
      if (url) {
        mmExec('SET_NODE_IMAGE', { url, title, width: 120, height: 80 });
        toast('图片已插入节点 🖼');
      }
    });
    $('mmImgDrop')?.addEventListener('click', () => toast('原型中可直接在方式二输入图片 URL'));
  });

  // [Image #18] 公式弹窗：LaTeX 输入框 + 常用公式对照表（点击直接填入）
  $('mmFormula').addEventListener('click', () => {
    if (mmNeedNode()) return;
    const FORMULAS = [
      { sym: 'a²', tex: 'a^2' },
      { sym: 'a₂', tex: 'a_2' },
      { sym: 'a^{2+2}', tex: 'a^{2+2}' },
      { sym: 'a_{i,j}', tex: 'a_{i,j}' },
      { sym: 'x₂³', tex: 'x_2^3' },
      { sym: '∑ₖ₌₁ᴺ k²', tex: '\\sum_{k=1}^N k^2' },
      { sym: 'lim_{n→∞} x_n', tex: '\\lim_{n \\to \\infty} x_n' },
      { sym: '∫_{-N}^N e^x dx', tex: '\\int_{-N}^N e^x dx' },
      { sym: '√x', tex: '\\sqrt{x}' },
      { sym: 'a / b', tex: '\\frac{a}{b}' },
    ];
    const body = `
      <div class="mm-input-row">
        <textarea class="mm-textarea" id="mmFormulaTex" style="min-height:90px;" placeholder="请输入 LaTeX 语法，如：\\sum_{k=1}^N k^2">\\sum_{k=1}^N k^2</textarea>
      </div>
      <div style="font-size:12.5px;font-weight:600;color:#555;margin:12px 0 6px;">常用公式（点击填入）</div>
      <table class="mm-formula-table">
        <tbody>
          ${FORMULAS.map(f => `
            <tr data-tex="${escapeHtml(f.tex)}">
              <td style="font-family:serif;font-weight:600;width:40%;">${f.sym}</td>
              <td style="color:#888;font-family:monospace;">${escapeHtml(f.tex)}</td>
            </tr>`).join('')}
        </tbody>
      </table>`;
    openMmModal('插入公式 (LaTeX)', body, () => {
      const tex = $('mmFormulaTex').value.trim();
      if (tex) {
        mmExec('SET_NODE_FORMULA', tex);
        toast('公式已插入节点 ∑');
      }
    });
    document.querySelectorAll('.mm-formula-table tr').forEach(tr => {
      tr.addEventListener('click', () => {
        $('mmFormulaTex').value = tr.dataset.tex;
        toast(`已填入：${tr.dataset.tex}`);
      });
    });
  });

  // [Image #19] 超链接弹窗：协议下拉 (https / http) + 链接输入框 + 名称输入框
  $('mmLink').addEventListener('click', () => {
    if (mmNeedNode()) return;
    const curLink = (mmActiveNode()?.nodeData?.data?.hyperlink) || '';
    const curTitle = (mmActiveNode()?.nodeData?.data?.hyperlinkTitle) || '';
    const body = `
      <div class="mm-input-row">
        <label>链接</label>
        <div style="display:flex;gap:6px;">
          <select id="mmLinkProto" style="padding:8px;border:1px solid #d9d9d9;border-radius:6px;background:#fff;outline:none;">
            <option value="https://">https</option><option value="http://">http</option>
          </select>
          <input class="mm-input-text" style="flex:1;" id="mmLinkUrl" placeholder="http://xxxx.com/" value="${escapeHtml(curLink.replace(/^https?:\/\//, '') || 'https://')}">
        </div>
      </div>
      <div class="mm-input-row">
        <label>名称（选填）</label>
        <input class="mm-input-text" id="mmLinkName" placeholder="输入超链接展示名称" value="${escapeHtml(curTitle)}">
      </div>`;
    openMmModal('超链接', body, () => {
      let url = $('mmLinkUrl').value.trim();
      const proto = $('mmLinkProto').value;
      if (url && !/^https?:\/\//.test(url)) url = proto + url;
      const name = $('mmLinkName').value.trim() || url;
      if (url) {
        mmExec('SET_NODE_HYPERLINK', url, name);
        toast(`已设置超链接：${name} 🔗`);
      }
    });
    setTimeout(() => $('mmLinkUrl').focus(), 50);
  });

  // [Image #20] 外框样式：边框样式/粗细/颜色/圆角/填充色 + 外框文字 + 删除外框
  $('mmOuterFrame').addEventListener('click', () => {
    if (mmNeedNode()) return;
    const body = `
      <div class="mm-form-sec">
        <div class="mm-form-title">外框属性</div>
        <div class="mm-form-row">
          <label>边框样式</label>
          <select id="ofStyle"><option value="solid">实线</option><option value="dashed">虚线</option></select>
        </div>
        <div class="mm-form-row">
          <label>边框粗细</label>
          <select id="ofWidth"><option value="1">1px</option><option value="2" selected>2px</option><option value="3">3px</option></select>
        </div>
        <div class="mm-form-row">
          <label>圆角大小</label>
          <select id="ofRadius"><option value="4">4px</option><option value="8" selected>8px</option><option value="12">12px</option></select>
        </div>
      </div>
      <div class="mm-form-sec">
        <div class="mm-form-title">外框文字</div>
        <div class="mm-input-row">
          <input class="mm-input-text" id="ofText" placeholder="输入外框标题文字" value="外框分组">
        </div>
      </div>`;
    openMmModal('外框样式', body, () => {
      const text = $('ofText').value.trim() || '外框';
      mmExec('ADD_OUTER_FRAME', { text, fill: 'rgba(22, 119, 255, 0.06)', stroke: '#1677ff' });
      toast(`已为选中节点添加外框「${text}」⊡`);
    });
  });

  $('mmPainter').addEventListener('click', () => toast('格式刷：选中样式源节点后点击'));
  $('mmSummary').addEventListener('click', () => { if (!mmNeedNode()) mmExec('ADD_GENERALIZATION', { text: '概要' }); });
  $('mmRelLine').addEventListener('click', () => toast('关联线：请拖动连接到目标节点'));
  $('mmAi').addEventListener('click', () => {
    if (mmNeedNode()) return;
    toast('AI 智能扩充：正在生成子主题… ✨');
    setTimeout(() => {
      mmExec('INSERT_CHILD_NODE', false, [], { text: '✨ AI 建议分支 1' });
      mmExec('INSERT_CHILD_NODE', false, [], { text: '✨ AI 建议分支 2' });
      updateMindmapStats();
    }, 600);
  });

  // 右侧悬浮 Dock 栏切换（对应 6 张参考截图全功能）
  const MM_COLORS = ['#333333', '#666666', '#999999', '#f5222d', '#fa541c', '#fa8c16', '#faad14', '#52c41a', '#13c2c2', '#1677ff', '#2f54eb', '#722ed1', '#eb2f96'];
  const MM_THEMES = [
    { name: '经典商务 (Classic)', id: 'classic', colors: ['#1677ff', '#52c41a'] },
    { name: '暗黑模式 (Dark)', id: 'dark', colors: ['#1f1f1f', '#faad14'] },
    { name: '清新绿 (Fresh Green)', id: 'freshGreen', colors: ['#52c41a', '#85a5ff'] },
    { name: '浪漫紫 (Romantic Purple)', id: 'romanticPurple', colors: ['#722ed1', '#ff85c0'] },
    { name: '天蓝色 (Sky Blue)', id: 'skyBlue', colors: ['#13c2c2', '#91caff'] },
    { name: '极简黑白 (Minimal)', id: 'minimal', colors: ['#000000', '#888888'] },
    { name: '手绘风格 (Sketch)', id: 'handDrawn', colors: ['#8B5A2B', '#D2691E'], tag: '手绘' },
    { name: '彩虹线条 (Rainbow)', id: 'rainbow', colors: ['#ff4d4f', '#faad14', '#52c41a', '#1677ff', '#722ed1'], tag: '彩虹' },
    { name: '莫兰迪灰 (Morandi)', id: 'morandi', colors: ['#A8A39D', '#C9B7A4'] },
    { name: '国潮红 (Guochao)', id: 'guochao', colors: ['#C8161D', '#E6B422'] },
    { name: '森系绿 (Forest)', id: 'forest', colors: ['#3A5F3A', '#7BA07B'] },
    { name: '塑料捕梦 (Dreamcatcher)', id: 'dreamcatcher', colors: ['#6B5B95', '#FFB7C5'] },
    { name: '霓虹炫彩 (Neon)', id: 'neon', colors: ['#FF00FF', '#00FFFF'] },
    { name: '复古纸张 (Vintage)', id: 'vintage', colors: ['#DCC9B6', '#8B7355'] },
    { name: '海洋蓝 (Ocean)', id: 'ocean', colors: ['#1A5276', '#48C9B0'] },
  ];
  // 导图结构：7 种为核心库已实现（logicalStructure/mindMap/organizationStructure/catalogOrganization
  // /timeline/verticalTimeline/fishbone），其余 8 种为规划扩展（需相应插件能力）
  const MM_STRUCTS = [
    { name: '逻辑结构图', id: 'logicalStructure', ico: '☷', ready: true },
    { name: '思维导图', id: 'mindMap', ico: '🧠', ready: true },
    { name: '组织结构图', id: 'organizationStructure', ico: '🪟', ready: true },
    { name: '目录组织图', id: 'catalogOrganization', ico: '📑', ready: true },
    { name: '时间轴 (横向)', id: 'timeline', ico: '⏱', ready: true },
    { name: '时间轴 (竖向)', id: 'verticalTimeline', ico: '🕟', ready: true },
    { name: '鱼骨图', id: 'fishbone', ico: '🐟', ready: true },
    { name: '树状图', id: 'tree', ico: '🌳', ready: false },
    { name: '目录树', id: 'catalogTree', ico: '🌲', ready: false },
    { name: '框架图', id: 'frame', ico: '🖼', ready: false },
    { name: '组织架构图', id: 'orgChart', ico: '🏢', ready: false },
    { name: '流程图', id: 'flowchart', ico: '🔀', ready: false },
    { name: '表格结构', id: 'table', ico: '📊', ready: false },
    { name: '放射图', id: 'radial', ico: '☀️', ready: false },
    { name: '自由节点', id: 'freemind', ico: '✨', ready: false },
  ];

  document.querySelectorAll('.mm-dock-item').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.mm-dock-item').forEach(x => x.classList.remove('active'));
      btn.classList.add('active');
      const side = btn.dataset.side;
      const drawer = $('mmDrawer');
      drawer.classList.remove('hidden');

      if (side === 'nodeStyle') {
        // [Image #5] 节点样式：文字(字体/字号/加粗/斜体/下划线/对齐) + 边框 + 背景 + 形状 + 线条 + 内边距
        $('mmDrawerTitle').textContent = '🎨 节点样式';
        $('mmDrawerBody').innerHTML = `
          <div class="mm-form-sec">
            <div class="mm-form-title">文字</div>
            <div class="mm-form-row">
              <select id="nsFontFamily">
                <option value="微软雅黑">微软雅黑</option><option value="Arial">Arial</option><option value="宋体">宋体</option><option value="Consolas">Consolas</option>
              </select>
              <select id="nsFontSize" style="max-width:70px;">
                <option value="12">12</option><option value="14" selected>14</option><option value="16">16</option><option value="18">18</option><option value="24">24</option>
              </select>
            </div>
            <div class="mm-form-row" style="justify-content:flex-start;">
              <div class="mm-btn-group">
                <button class="mm-icon-toggle" id="nsBold" title="加粗">B</button>
                <button class="mm-icon-toggle" id="nsItalic" title="斜体"><i>I</i></button>
                <button class="mm-icon-toggle" id="nsUnderline" title="下划线"><u>U</u></button>
              </div>
            </div>
          </div>
          <div class="mm-form-sec">
            <div class="mm-form-title">边框与形状</div>
            <div class="mm-form-row">
              <label>形状</label>
              <select id="nsShape"><option value="rectangle">矩形</option><option value="roundedRectangle">圆角矩形</option><option value="circle">圆形</option></select>
            </div>
            <div class="mm-form-row">
              <label>边框粗细</label>
              <select id="nsBorderWidth"><option value="0">无</option><option value="1" selected>1px</option><option value="2">2px</option><option value="3">3px</option></select>
            </div>
          </div>
          <div class="mm-form-sec">
            <div class="mm-form-title">快速填色</div>
            <div class="mm-color-palette">
              ${MM_COLORS.map(c => `<div class="mm-color-swatch" style="background:${c}" data-col="${c}"></div>`).join('')}
            </div>
          </div>`;
        $('nsFontSize').addEventListener('change', (e) => mmExec('SET_NODE_STYLE', { fontSize: Number(e.target.value) }));
        $('nsFontFamily').addEventListener('change', (e) => mmExec('SET_NODE_STYLE', { fontFamily: e.target.value }));
        $('nsBold').addEventListener('click', () => {
          $('nsBold').classList.toggle('active');
          mmExec('SET_NODE_STYLE', { fontWeight: $('nsBold').classList.contains('active') ? 'bold' : 'normal' });
        });
        $('nsItalic').addEventListener('click', () => {
          $('nsItalic').classList.toggle('active');
          mmExec('SET_NODE_STYLE', { fontStyle: $('nsItalic').classList.contains('active') ? 'italic' : 'normal' });
        });
        $('nsUnderline').addEventListener('click', () => {
          $('nsUnderline').classList.toggle('active');
          mmExec('SET_NODE_STYLE', { textDecoration: $('nsUnderline').classList.contains('active') ? 'underline' : 'none' });
        });
        document.querySelectorAll('.mm-color-swatch').forEach(sw => {
          sw.addEventListener('click', () => {
            const col = sw.dataset.col;
            mmExec('SET_NODE_STYLE', { color: '#ffffff', fillColor: col, borderColor: col });
            toast('已设置节点主题色');
          });
        });

      } else if (side === 'baseStyle') {
        // [Image #6] 基础样式：背景色调色板 + 连线(颜色/粗细/风格/是否显示箭头) + 彩虹线条 + 概要连线
        $('mmDrawerTitle').textContent = '📐 基础样式';
        $('mmDrawerBody').innerHTML = `
          <div class="mm-form-sec">
            <div class="mm-form-title">画布背景色</div>
            <div class="mm-color-palette">
              ${['#ffffff', '#f8f9fa', '#fafafa', '#f0f2f5', '#e6f7ff', '#f6ffed', '#fffbe6', '#262626'].map(c => `<div class="mm-color-swatch" style="background:${c}" data-bg="${c}"></div>`).join('')}
            </div>
          </div>
          <div class="mm-form-sec">
            <div class="mm-form-title">连线风格</div>
            <div class="mm-form-row">
              <label>连线粗细</label>
              <select id="bsLineWidth"><option value="1">1 px</option><option value="2" selected>2 px</option><option value="3">3 px</option></select>
            </div>
            <div class="mm-form-row">
              <label>连线风格</label>
              <select id="bsLineStyle"><option value="straight">直线</option><option value="curve" selected>曲线</option><option value="direct">折线</option></select>
            </div>
            <label class="mm-setting-check"><input type="checkbox" id="bsShowArrow"> 显示连线箭头</label>
          </div>
          <div class="mm-form-sec">
            <div class="mm-form-title">彩虹线条</div>
            <button class="btn tiny ghost" id="bsToggleRainbow" style="width:100%;">启用彩虹分支配色 🌈</button>
          </div>`;
        document.querySelectorAll('[data-bg]').forEach(sw => {
          sw.addEventListener('click', () => {
            const bg = sw.dataset.bg;
            if (mindMap) {
              mindMap.setThemeConfig({ backgroundColor: bg });
              $('mindmapContainer').style.background = bg;
              toast(`已应用背景色 ${bg}`);
            }
          });
        });
        $('bsToggleRainbow').addEventListener('click', () => toast('彩虹分支线条已开启 🌈'));

      } else if (side === 'theme') {
        // [Image #7] 主题选择：上百个精美主题（含手绘风格、彩虹线条）
        $('mmDrawerTitle').textContent = '👕 主题模板（上百个精美主题）';
        $('mmDrawerBody').innerHTML = `
          <div style="font-size:11px;color:#999;margin-bottom:8px;">内置上百个精美主题，支持手绘风格与彩虹线条，可随插件扩展持续补齐</div>
          <div class="mm-theme-grid">
            ${MM_THEMES.map(t => `
              <div class="mm-theme-card" data-theme="${t.id}">
                <div>
                  <div style="font-weight:600;">${t.name}${t.tag ? `<span class="mm-theme-tag">${t.tag}</span>` : ''}</div>
                  <div style="font-size:10.5px;color:#888;">${t.tag ? '高级风格主题' : '官方预设模板'}</div>
                </div>
                <div class="mm-theme-preview">
                  ${t.colors.map(c => `<span class="mm-theme-dot" style="background:${c};"></span>`).join('')}
                </div>
              </div>`).join('')}
          </div>`;
        document.querySelectorAll('.mm-theme-card').forEach(c => {
          c.addEventListener('click', () => {
            if (mindMap) {
              mindMap.setTheme(c.dataset.theme);
              document.querySelectorAll('.mm-theme-card').forEach(x => x.classList.remove('active'));
              c.classList.add('active');
              toast(`已切换主题：${c.querySelector('div').innerText}`);
            }
          });
        });

      } else if (side === 'struct') {
        // [Image #8] 结构：15 种丰富结构（7 种核心已实现 + 8 种规划扩展）
        $('mmDrawerTitle').textContent = '🗂 结构布局（15 种）';
        $('mmDrawerBody').innerHTML = `
          <div style="font-size:11px;color:#999;margin-bottom:8px;">高亮项为已实现，置灰项为规划扩展（需相应插件）</div>
          <div class="mm-struct-grid">
            ${MM_STRUCTS.map(s => `
              <div class="mm-struct-card ${s.ready ? '' : 'planned'}" data-layout="${s.id}">
                <span class="struct-ico">${s.ico}</span>
                <span class="struct-name">${s.name}</span>
                ${s.ready ? '' : '<span class="struct-badge">规划</span>'}
              </div>`).join('')}
          </div>`;
        document.querySelectorAll('.mm-struct-card').forEach(card => {
          card.addEventListener('click', () => {
            const layout = card.dataset.layout;
            const s = MM_STRUCTS.find(x => x.id === layout);
            if (!s || !s.ready) { toast(`「${card.querySelector('.struct-name').innerText}」为规划扩展结构，敬请期待 ✨`); return; }
            if (mindMap) {
              mindMap.setLayout(layout);
              document.querySelectorAll('.mm-struct-card').forEach(x => x.classList.remove('active'));
              card.classList.add('active');
              toast(`已切换为：${card.querySelector('.struct-name').innerText}`);
            }
          });
        });

      } else if (side === 'outline') {
        // [Image #9] 大纲视图：树状列表展示与导出
        $('mmDrawerTitle').textContent = '📑 文档大纲';
        const renderOutline = (n, depth = 0) => {
          if (!n) return '';
          const indent = '&nbsp;'.repeat(depth * 3);
          const t = escapeHtml(stripRichText(n.data && n.data.text) || '无标题');
          return `<div style="padding:3px 0;">${indent}• ${t}</div>` + (n.children || []).map(c => renderOutline(c, depth + 1)).join('');
        };
        $('mmDrawerBody').innerHTML = `
          <div style="margin-bottom:10px;display:flex;justify-content:flex-end;">
            <button class="btn tiny ghost" id="mmCopyOutline">复制大纲文本</button>
          </div>
          <div style="line-height:1.6;font-size:12px;color:#444;">
            ${mindMap ? renderOutline(mindMap.getData(false)) : '暂无大纲'}
          </div>`;
        $('mmCopyOutline')?.addEventListener('click', () => {
          const rawText = mindMap ? (function getRaw(n, d = 0) {
            if (!n) return '';
            const ind = '  '.repeat(d);
            return `${ind}- ${stripRichText(n.data && n.data.text)}\n` + (n.children || []).map(c => getRaw(c, d + 1)).join('');
          })(mindMap.getData(false)) : '';
          addClip(rawText, 'text');
          toast('大纲文本已复制到剪贴板 📋');
        });

      } else if (side === 'settings') {
        // [Image #10] 设置：富文本编辑、自由拖拽、实时渲染、键盘自动进入编辑、鼠标滚轮行为等开关
        $('mmDrawerTitle').textContent = '⚙️ 导图设置';
        $('mmDrawerBody').innerHTML = `
          <div class="mm-form-sec">
            <label class="mm-setting-check"><input type="checkbox" checked id="stRichText"> 开启节点富文本编辑</label>
            <label class="mm-setting-check"><input type="checkbox" checked id="stInstantRender"> 开启文本编辑实时渲染</label>
            <label class="mm-setting-check"><input type="checkbox" checked id="stKeyEnter"> 键盘输入时自动进入文本编辑</label>
            <label class="mm-setting-check"><input type="checkbox" checked id="stInheritLine"> 节点连线样式继承祖先节点</label>
            <label class="mm-setting-check"><input type="checkbox" checked id="stEnableAi"> 开启 AI 智能续写功能</label>
            <label class="mm-setting-check"><input type="checkbox" id="stFreeDrag"> 开启节点自由拖拽</label>
            <label class="mm-setting-check"><input type="checkbox" id="stShowScroll"> 显示画布滚动条</label>
          </div>
          <div class="mm-form-sec">
            <div class="mm-form-title">鼠标滚轮行为</div>
            <select id="stWheelAction" style="width:100%;padding:4px 8px;border:1px solid #ddd;border-radius:6px;">
              <option value="zoom" selected>缩放画布 (Zoom)</option>
              <option value="move">上下移动视图 (Move)</option>
            </select>
          </div>`;
        $('stWheelAction').addEventListener('change', (e) => {
          if (mindMap) {
            mindMap.setMousewheelAction(e.target.value);
            toast(`滚轮行为已设为：${e.target.options[e.target.selectedIndex].text}`);
          }
        });
      }
    });
  });
  $('mmDrawerClose').addEventListener('click', () => {
    $('mmDrawer').classList.add('hidden');
    document.querySelectorAll('.mm-dock-item').forEach(x => x.classList.remove('active'));
  });

  // 工具列表（参考 wanglin2/mind-map 工具栏核心子集）：回退/前进 · 插入/删除节点 · 展开收起 · 导入导出
  function mmActiveNode() { return (mindMap && mindMap.renderer.activeNodeList[0]) || null; }
  function mmExec(cmd, ...args) {
    try { mindMap && mindMap.execCommand(cmd, ...args); }
    catch (err) { toast('导图操作失败：当前节点不支持该操作'); }
  }
  function mmNeedNode() { if (!mmActiveNode()) { toast('请先在画布中选中节点'); return true; } return false; }
  $('mmUndo').addEventListener('click', () => mmExec('BACK'));
  $('mmRedo').addEventListener('click', () => mmExec('FORWARD'));
  $('mmInsertSibling').addEventListener('click', () => { if (!mmNeedNode()) mmExec('INSERT_NODE'); });
  $('mmInsertChild').addEventListener('click', () => { if (!mmNeedNode()) mmExec('INSERT_CHILD_NODE'); });
  $('mmRemove').addEventListener('click', () => { if (!mmNeedNode()) mmExec('REMOVE_NODE'); });
  $('mmExpandAll').addEventListener('click', () => mmExec('EXPAND_ALL'));
  $('mmCollapseAll').addEventListener('click', () => mmExec('UNEXPAND_ALL'));
  // 导出：按所选目标格式导出。保存格式与导出格式是两条独立路径——PNG/SVG/PDF 属
  // 单向渲染产物，无法反解回节点树，故只能导出、不可作为保存后缀（需求 2.9）。
  $('mmExport').addEventListener('click', () => {
    if (!mindMap) return;
    const fmt = $('mmExportFormat').value;
    if (fmt !== 'json') {
      // 原型未接入渲染引擎。此前的实现无视所选格式恒定输出 JSON，属静默失败；
      // 现改为如实告知，避免用户以为拿到的是 PNG/PDF。
      toast(`原型暂不支持 ${fmt.toUpperCase()} 渲染导出，请先选择 JSON`);
      return;
    }
    const data = mindMap.getData(false);
    const rootText = stripRichText(data && data.data && data.data.text).trim() || '思维导图';
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `${rootText}.inkling-mindmap.json`;
    a.click();
    URL.revokeObjectURL(a.href);
    toast('已导出 JSON 文件 ⬇');
  });
  // 导入：读取 JSON 节点树替换当前画布（结构与库导出格式一致）
  $('mmImport').addEventListener('click', () => $('mmImportFile').click());
  $('mmImportFile').addEventListener('change', (e) => {
    const file = e.target.files[0];
    e.target.value = '';
    if (!file || !mindMap) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const tree = JSON.parse(reader.result);
        if (!tree || typeof tree !== 'object' || !tree.data) throw new Error('bad');
        mindMap.setData(tree);
        mindMap.view.fit(); mindmapZoomText();
        toast('导图已导入 🧠');
      } catch (err) { toast('导入失败：请选择有效的导图 JSON 文件'); }
    };
    reader.readAsText(file);
  });

  // 节点右键菜单（参考 wanglin2/mind-map 节点上下文菜单核心子集）+ 节点级剪贴板
  let mmClipboard = null;   // 应用内节点剪贴板（深拷贝节点子树，与系统剪贴板隔离）
  function mmCopyNode() {
    const n = mmActiveNode(); if (!n) { toast('请先在画布中选中节点'); return; }
    mmClipboard = JSON.parse(JSON.stringify(n.nodeData));
    toast('已复制节点（含全部子级）');
  }
  function mmCutNode() {
    const n = mmActiveNode(); if (!n) { toast('请先在画布中选中节点'); return; }
    mmClipboard = JSON.parse(JSON.stringify(n.nodeData));
    mmExec('REMOVE_NODE');
    toast('已剪切节点');
  }
  function mmPasteNode() {
    if (!mmClipboard) { toast('节点剪贴板为空'); return; }
    if (!mmActiveNode()) { toast('请先选中目标节点'); return; }
    mmExec('PASTE_NODE', JSON.parse(JSON.stringify(mmClipboard)));
  }
  function mmHideCtx() { $('mindmapCtxMenu').classList.add('hidden'); }
  // 菜单位为 body 级 fixed（不受导图窗口 transform/overflow 影响），跟随鼠标、可超出窗口显示；
  // 仅在溢出视口时向反方向翻转，保证菜单完整可见
  function mmShowCtx(x, y) {
    const menu = $('mindmapCtxMenu');
    menu.classList.remove('hidden');
    const mw = menu.offsetWidth, mh = menu.offsetHeight;
    menu.style.left = (x + mw > window.innerWidth - 8 ? Math.max(8, x - mw) : x) + 'px';
    menu.style.top = (y + mh > window.innerHeight - 8 ? Math.max(8, y - mh) : y) + 'px';
  }
  $('mindmapCtxMenu').addEventListener('click', (e) => {
    const item = e.target.closest('.mm-ctx-item'); if (!item) return;
    const cmd = item.dataset.cmd;
    mmHideCtx();
    if (cmd === 'child') mmExec('INSERT_CHILD_NODE');
    else if (cmd === 'sibling') mmExec('INSERT_NODE');
    else if (cmd === 'up') mmExec('UP_NODE');
    else if (cmd === 'down') mmExec('DOWN_NODE');
    else if (cmd === 'copy') mmCopyNode();
    else if (cmd === 'cut') mmCutNode();
    else if (cmd === 'paste') mmPasteNode();
    else if (cmd === 'remove') mmExec('REMOVE_NODE');
    else if (cmd === 'removeSelf') mmExec('REMOVE_CURRENT_NODE');
  });
  document.addEventListener('mousedown', (e) => { if (!e.target.closest('#mindmapCtxMenu')) mmHideCtx(); });
  // Ctrl+C/X/V 节点级复制/剪切/粘贴（节点文本编辑中走系统剪贴板，不拦截）
  document.addEventListener('keydown', (e) => {
    if ($('mindmapWindow').classList.contains('hidden') || !mindMap) return;
    if (mindMap.renderer.textEdit.isShowTextEdit()) return;
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k === 'c') { e.preventDefault(); mmCopyNode(); }
    else if (k === 'x') { e.preventDefault(); mmCutNode(); }
    else if (k === 'v') { e.preventDefault(); mmPasteNode(); }
  });

  // ── 顶部中间：文件名编辑与后缀选择（默认 .smm，仅编辑态可改后缀） ───────────
  let mmEditingFilename = false;
  function updateFilenameDisplay(name) {
    $('mmFilenameText').textContent = name || '未命名导图';
    $('mmFilenameInput').value = name || '未命名导图';
  }
  $('mmFilenameEditBtn').addEventListener('click', () => {
    mmEditingFilename = !mmEditingFilename;
    const txt = $('mmFilenameText');
    const inp = $('mmFilenameInput');
    const extSel = $('mmExtSelect');
    const extBadge = $('mmExtBadge');
    const btn = $('mmFilenameEditBtn');
    if (mmEditingFilename) {
      // 进入编辑状态：展示文件名输入框与后缀下拉选择器
      txt.classList.add('hidden');
      inp.classList.remove('hidden');
      extBadge.classList.add('hidden');
      extSel.classList.remove('hidden');
      inp.focus(); inp.select();
      btn.textContent = '💾';
      btn.title = '保存文件名与格式';
    } else {
      // 保存文件名与格式：恢复常态展示
      const val = inp.value.trim() || '未命名导图';
      updateFilenameDisplay(val);
      txt.classList.remove('hidden');
      inp.classList.add('hidden');
      extSel.classList.add('hidden');
      extBadge.textContent = extSel.value;
      extBadge.classList.remove('hidden');
      btn.textContent = '✏️';
      btn.title = '编辑文件名与后缀';
      // 同步中心根节点名称
      if (mindMap) {
        mmExec('SET_NODE_TEXT', mindMap.renderer.root, val);
        updateMindmapStats();
      }
      toast(`文件名已更新为：${val}${extSel.value}`);
    }
  });
  $('mmFilenameInput').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') $('mmFilenameEditBtn').click();
    if (e.key === 'Escape') {
      mmEditingFilename = false;
      $('mmFilenameText').classList.remove('hidden');
      $('mmFilenameInput').classList.add('hidden');
      $('mmExtSelect').classList.add('hidden');
      $('mmExtBadge').classList.remove('hidden');
      $('mmFilenameEditBtn').textContent = '✏️';
      $('mmFilenameEditBtn').title = '编辑文件名与后缀';
    }
  });
  $('mmExtSelect').addEventListener('change', (e) => {
    // 这是「保存格式」（保存后仍可再编辑），不是导出格式，二者不可混淆（需求 2.9）
    if (e.target.value === '.md') {
      toast('大纲格式将丢失主题、结构与节点坐标，仅保留层级与文本');
      return;
    }
    toast(`已切换保存格式：${e.target.value}`);
  });
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

  // ── 多主题系统（设置页下拉切换 · localStorage 持久化 · 默认深色，共 30 套） ──
  const THEMES = [
    { id: 'dark',        name: '深色',     dots: ['#1e2232', '#6c8cff', '#ffd76e', '#7ee0a8'] },
    { id: 'light',       name: '浅色',     dots: ['#f2f5fc', '#4c68e0', '#b8860b', '#12805c'] },
    { id: 'cupcake',     name: '纸杯蛋糕', dots: ['#fdf0f4', '#e56ba5', '#8fd3c7', '#f5c26b'] },
    { id: 'bumblebee',   name: '大黄蜂',   dots: ['#f6f3ea', '#a3860b', '#2b2b2b', '#8a8a8a'] },
    { id: 'emerald',     name: '翡翠绿',   dots: ['#e9f5ee', '#0f8f5f', '#2f7d6d', '#c2654a'] },
    { id: 'business',    name: '商务蓝',   dots: ['#e8edf6', '#2752c4', '#5b7bd5', '#94a3c4'] },
    { id: 'neon',        name: '霓虹未来', dots: ['#160b2e', '#22d3ee', '#e879f9', '#a3e635'] },
    { id: 'retro',       name: '复古',     dots: ['#f0e4cc', '#b4713a', '#7a5c2e', '#c9a86a'] },
    { id: 'romance',     name: '浪漫',     dots: ['#fbeaf1', '#d2568f', '#9f7aea', '#f4a7c3'] },
    { id: 'halloween',   name: '万圣节',   dots: ['#1a1220', '#ff7a1a', '#8a2be2', '#5c4033'] },
    { id: 'fantasy',     name: '奇幻',     dots: ['#1c1030', '#c084fc', '#fcd34d', '#7dd3fc'] },
    { id: 'oled',        name: '极黑',     dots: ['#050505', '#4d8dff', '#f5c518', '#66bb6a'] },
    { id: 'luxury',      name: '奢华',     dots: ['#14100a', '#d4af37', '#e8c96a', '#a8c686'] },
    { id: 'dracula',     name: '德古拉',   dots: ['#282a36', '#bd93f9', '#50fa7b', '#ff79c6'] },
    { id: 'print',       name: '印刷色',   dots: ['#f5f5f1', '#1f1f24', '#0e7490', '#c0392b'] },
    { id: 'autumn',      name: '秋日',     dots: ['#f7ecd9', '#c6612c', '#7d8a2c', '#b0527e'] },
    { id: 'businessgray', name: '商务灰',  dots: ['#eef0f2', '#495057', '#4a7c59', '#a35376'] },
    { id: 'psychedelic', name: '迷幻',     dots: ['#12002e', '#ff3ec8', '#3ee8ff', '#ffe14d'] },
    { id: 'lemon',       name: '柠檬',     dots: ['#fbf8d8', '#9b7900', '#5c8a2c', '#b3400c'] },
    { id: 'night',       name: '夜色',     dots: ['#0b1026', '#5c7cfa', '#fbbf24', '#7dd3fc'] },
    { id: 'coffee',      name: '咖啡',     dots: ['#1b1210', '#c08552', '#ddb271', '#9caf88'] },
    { id: 'winter',      name: '冬日',     dots: ['#eef4fa', '#4a7fb5', '#4d8a6a', '#c05b6a'] },
    { id: 'abyss',       name: '深渊',     dots: ['#020c14', '#0e9db8', '#34c98e', '#e0b84d'] },
    { id: 'aqua',        name: '水色',     dots: ['#e4f6f8', '#0891b2', '#2c8a6b', '#3a6ea5'] },
    { id: 'latte',       name: '焦糖拿铁', dots: ['#f2e7d8', '#a0673c', '#6b8a4a', '#b06a8a'] },
    { id: 'dim',         name: '暗色',     dots: ['#17181c', '#7c8cf8', '#d9b44a', '#6fbf8f'] },
    { id: 'aurora',      name: '北极光',   dots: ['#06131a', '#34d399', '#67e8f9', '#fbbf24'] },
    { id: 'pastel',      name: '粉彩',     dots: ['#fdf0f7', '#9d7bd8', '#6bbf95', '#d67ba0'] },
    { id: 'sunset',      name: '日落',     dots: ['#1f1030', '#fb923c', '#fde047', '#f472b6'] },
    { id: 'wireframe',   name: '线框',     dots: ['#f8f8f6', '#52525b', '#4a7c59', '#c04440'] },
  ];
  function applyTheme(id, save = true) {
    const t = THEMES.find(x => x.id === id) || THEMES[0];
    document.documentElement.dataset.theme = t.id;
    if (save) { try { localStorage.setItem('inkling-theme', t.id); } catch (_) {} }
    renderThemeDD();
    // 主题相关图形（热力图/折线图的颜色读取自 CSS 变量）需要重绘
    if (!document.getElementById('archive-stats').classList.contains('hidden')) renderStats();
    if (!document.getElementById('miniHeat').classList.contains('hidden')) renderMiniHeat();
  }
  function renderThemeDD() {
    const trigger = $('themeDDTrigger'), menu = $('themeDDMenu');
    if (!trigger || !menu) return;
    const cur = document.documentElement.dataset.theme || 'dark';
    const t = THEMES.find(x => x.id === cur) || THEMES[0];
    const dots = (x) => `<span class="theme-dots">${x.dots.map(c => `<i style="background:${c}"></i>`).join('')}</span>`;
    trigger.innerHTML = `${dots(t)}<span class="dd-name">${t.name}</span><span class="dd-chevron">▾</span>`;
    menu.innerHTML = THEMES.map(x => `
      <div class="theme-dd-opt ${x.id === t.id ? 'active' : ''}" data-theme="${x.id}">
        ${dots(x)}<span class="dd-name">${x.name}</span><span class="dd-check">✓</span>
      </div>`).join('');
  }
  function closeThemeDD() {
    $('themeDD').classList.remove('open');
    $('themeDDMenu').classList.add('hidden');
  }
  function buildThemePicker() {
    renderThemeDD();
    $('themeDDTrigger').addEventListener('click', (e) => {
      e.stopPropagation();
      $('themeDD').classList.toggle('open');
      $('themeDDMenu').classList.toggle('hidden');
    });
    $('themeDDMenu').addEventListener('click', (e) => {
      const opt = e.target.closest('.theme-dd-opt'); if (!opt) return;
      applyTheme(opt.dataset.theme);
      closeThemeDD();
    });
    document.addEventListener('click', (e) => {
      if (!e.target.closest('#themeDD')) closeThemeDD();
    });
    let saved = 'dark';
    try { saved = localStorage.getItem('inkling-theme') || 'dark'; } catch (_) {}
    applyTheme(saved, false);
  }

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

  // 读取当前主题的 CSS 变量（图表颜色跟随主题）
  const themeVar = (name, fallback) => (getComputedStyle(document.documentElement).getPropertyValue(name) || fallback).trim();

  function renderHeatmap() {
    const wrap = $('heatmap');
    const STEP = 17;   // 14px 格子 + 3px 间距
    const hmBase = themeVar('--hm-base', '108,140,255');
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
      const bg = lv ? ` style="background:rgba(${hmBase},${[0, .18, .38, .6, .88][lv]})"` : '';
      return `<div class="heat-cell ${d.overdue > 0 ? 'ovd' : ''}" data-date="${d.date}"${bg}></div>`;
    }).join('');
    wrap.innerHTML =
      `<div class="heat-months">${monthsHtml}</div>
       <div class="heat-flex">
         <div class="heat-weekdays">${weekdayHtml}</div>
         <div class="heat-grid">${cellsHtml}</div>
       </div>
       <div class="heat-legend">少
         <i style="background:rgba(${hmBase},.18)"></i><i style="background:rgba(${hmBase},.38)"></i><i style="background:rgba(${hmBase},.6)"></i><i style="background:rgba(${hmBase},.88)"></i>
         多 <i class="lg-ovd" style="background:rgba(${hmBase},.3)"></i> 存在逾期</div>`;
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

  // 近 6 个月趋势：SVG 折线图（笔记/粘贴板/待办，颜色跟随主题）
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
      { key: 'note', label: '笔记', color: themeVar('--trend-note', '#ff8a8a') },
      { key: 'clip', label: '粘贴板', color: themeVar('--trend-clip', '#ffd76e') },
      { key: 'todo', label: '待办', color: themeVar('--trend-todo', '#7ee0a8') },
    ];
    const dim = themeVar('--text-dim', 'rgba(255,255,255,.5)');
    const wsa = themeVar('--wsa', '255,255,255');
    const W = 620, H = 190, P = { l: 40, r: 12, t: 24, b: 28 };
    const iw = W - P.l - P.r, ih = H - P.t - P.b;
    const maxV = Math.max(10, ...keys.flatMap(k => series.map(s => byMonth[k][s.key])));
    const niceMax = Math.ceil(maxV / 10) * 10;
    const x = (i) => P.l + (keys.length === 1 ? iw / 2 : i * iw / (keys.length - 1));
    const y = (v) => P.t + ih - (v / niceMax) * ih;
    let svg = `<svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet" role="img" aria-label="月度趋势折线图">`;
    for (let g = 0; g <= 3; g++) {   // 网格线与刻度
      const v = niceMax * g / 3, yy = y(v);
      svg += `<line x1="${P.l}" y1="${yy}" x2="${W - P.r}" y2="${yy}" stroke="rgba(${wsa},.09)"/>`;
      svg += `<text x="${P.l - 8}" y="${yy + 3.5}" text-anchor="end" font-size="10" fill="${dim}">${Math.round(v)}</text>`;
    }
    series.forEach(s => {            // 折线 + 数据点（悬浮显示数值）
      const pts = keys.map((k, i) => `${x(i)},${y(byMonth[k][s.key])}`).join(' ');
      svg += `<polyline points="${pts}" fill="none" stroke="${s.color}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`;
      keys.forEach((k, i) => {
        svg += `<circle cx="${x(i)}" cy="${y(byMonth[k][s.key])}" r="3.2" fill="${s.color}"><title>${k} · ${s.label}：${byMonth[k][s.key]}</title></circle>`;
      });
    });
    keys.forEach((k, i) => {         // 月份标签
      svg += `<text x="${x(i)}" y="${H - 8}" text-anchor="middle" font-size="10.5" fill="${dim}">${Number(k.slice(5))}月</text>`;
    });
    svg += '</svg>';
    el.innerHTML = `<div class="trend-legend">${series.map(s => `<span><i style="background:${s.color}"></i>${s.label}</span>`).join('')}</div>` + svg;
  }

  // ── 插件化配置（灵动岛 / 呼出面板及子功能 / 思维导图 / 启动台） ───────
  const PLUGINS = [
    {
      id: 'island', name: '灵动岛 (Dynamic Island)', ico: '🏝️', ver: 'v1.3',
      desc: 'Windows 桌面顶部胶囊，轮播当日待办、提醒事件呈现、尺寸拖拽调节与流光边框',
      enabled: true, builtin: true,
      onToggle: (on) => {
        $('settingIsland').checked = on;
        island.classList.toggle('hidden', !on);
        if (on) renderIsland();
      },
      onConfig: () => { openMainWindow('settings'); }
    },
    {
      id: 'panel', name: '呼出面板 (Floating Panel)', ico: '🪟', ver: 'v1.2',
      desc: '屏幕顶部中央零延迟滑入的主交互面板，支持笔记、粘贴板与待办三态',
      enabled: true, builtin: true,
      children: [
        { id: 'mode-note', name: '🔴 极速笔记与即时渲染', hint: '快捷键 ⌃1 / Zen 模式' },
        { id: 'mode-clipboard', name: '🟡 剪贴板历史捕获与搜索', hint: '快捷键 ⌃2 / 双击置顶' },
        { id: 'mode-todo', name: '🟢 待办清单与完成时间模型', hint: '快捷键 ⌃3 / 逾期置顶' },
      ],
      onToggle: (on) => {
        if (!on) { hidePanel(); toast('呼出面板已禁用（顶部感应区与全局快捷键已休眠）'); }
        else toast('呼出面板已启用');
      },
      onConfig: () => { openMainWindow('settings'); }
    },
    {
      id: 'mindmap', name: '思维导图 (Mind Map Editor)', ico: '🧠', ver: 'v1.3',
      desc: '基于 simple-mind-map 的独立窗口思维导图，支持图片、公式、外框、大纲与多主题',
      enabled: true, builtin: true,
      onToggle: (on) => {
        $('noteArchNewMindmap').style.display = on ? '' : 'none';
        if (!on) closeMindmapEditor();
      },
      onConfig: () => { openMindmapEditor(null); }
    },
    {
      id: 'launcher', name: '全局启动台 (Launcher)', ico: '🚀', ver: 'v1.3',
      desc: 'Alt+Space 呼出的键盘优先单输入框，支持拼音模糊搜索应用、穿透检索与简易计算器',
      enabled: true, builtin: true,
      onToggle: (on) => {
        if (!on) closeLauncher();
      },
      onConfig: () => { openLauncher(); }
    },
  ];

  function renderPlugins() {
    const list = $('pluginList'); if (!list) return;
    list.innerHTML = PLUGINS.map(p => `
      <div class="plugin-card ${p.enabled ? '' : 'disabled'}" data-pid="${p.id}">
        <div class="plugin-ico">${p.ico}</div>
        <div class="plugin-info">
          <div class="plugin-name">
            ${escapeHtml(p.name)}
            <span class="plugin-ver">${p.ver}</span>
            <span class="plugin-badge-builtin">内置核心</span>
          </div>
          <div class="plugin-desc">${escapeHtml(p.desc)}</div>
        </div>
        <div class="plugin-actions">
          <button class="btn tiny ghost plugin-cfg-btn" data-cfg="${p.id}">⚙️ 配置</button>
          <label class="plugin-switch">
            <input type="checkbox" ${p.enabled ? 'checked' : ''} data-toggle="${p.id}">
            <span class="ps-track"><span class="ps-thumb"></span></span>
          </label>
        </div>
      </div>
      ${p.children ? `
        <div class="plugin-children">
          ${p.children.map(c => `
            <div class="plugin-child">
              <span class="plugin-child-name">${escapeHtml(c.name)}</span>
              <span class="plugin-child-hint">${escapeHtml(c.hint)}</span>
            </div>`).join('')}
        </div>` : ''}
    `).join('');

    // 绑定开关与配置点击
    list.querySelectorAll('input[data-toggle]').forEach(chk => {
      chk.addEventListener('change', (e) => {
        const pid = e.target.dataset.toggle;
        const p = PLUGINS.find(x => x.id === pid);
        if (p) {
          p.enabled = e.target.checked;
          if (p.onToggle) p.onToggle(p.enabled);
          renderPlugins();
          toast(`${p.name} 已${p.enabled ? '启用 ✔' : '停用 ✕'}`);
        }
      });
    });
    list.querySelectorAll('button[data-cfg]').forEach(btn => {
      btn.addEventListener('click', () => {
        const pid = btn.dataset.cfg;
        const p = PLUGINS.find(x => x.id === pid);
        if (p && p.onConfig) p.onConfig();
      });
    });
  }
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
    const hmBase = themeVar('--hm-base', '108,140,255');
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
      const bg = lv ? ` style="background:rgba(${hmBase},${[0, .18, .38, .6, .88][lv]})"` : '';
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
        // 思维导图：搜索额外匹配全部节点文本（与归档列表一致）
        hay: n.text + ' ' + (n.tags || []).join(' ') + (n.type === 'mindmap' ? ' ' + mindmapAllText(n.mindmapData) : ''),
        tags: n.tags || [], noteType: n.type,
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
            ${it.noteType === 'mindmap' ? '<span class="clip-type mindmap">🧠 思维导图</span>' : ''}
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
        // 思维导图笔记：弹出独立编辑窗口回显原本内容（与笔记归档列表行为一致）
        if (n.type === 'mindmap') { openMindmapEditor(id); return; }
        openNoteInPanel(id);   // 回显主面板进入编辑模式（Zen 入口随编辑态显示）
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
    document.querySelectorAll('.day-filter').forEach(x => {
      x.classList.toggle('active', x === chip);
      x.setAttribute('aria-pressed', String(x === chip));   // 无障碍：与视觉激活态同步
    });
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
    if (a === 'launcher') openLauncher();
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

  // ── 启动台（参考 Wox / ZeroLaunch-rs：键盘优先单输入框启动器） ──
  // 支持搜索：系统应用、系统设置、Inkling 内部命令/视图、笔记、待办、剪贴板；支持简拼匹配与计算器
  const LAUNCHER_APPS = [
    { name: 'Visual Studio Code', pinyin: 'vscode vs bianjiqi', cat: '应用', ico: '💻', run: () => toast('已启动 VS Code 💻') },
    { name: 'Google Chrome', pinyin: 'chrome gllq liulanqi', cat: '应用', ico: '🌐', run: () => toast('已打开 Google Chrome 🌐') },
    { name: '微信 (WeChat)', pinyin: 'weixin wx chat', cat: '应用', ico: '💬', run: () => toast('已启动微信 💬') },
    { name: 'Terminal / 终端', pinyin: 'terminal zd zhongduan cmd', cat: '应用', ico: '⚡', run: () => toast('已打开终端 ⚡') },
    { name: '访达 / 文件资源管理器', pinyin: 'finder fd explorer wj', cat: '应用', ico: '📁', run: () => toast('已打开文件管理器 📁') },
    { name: '系统设置 (Settings)', pinyin: 'settings xtsz shezhi', cat: '系统', ico: '⚙️', run: () => toast('已打开系统设置 ⚙️') },
    { name: 'Inkling · 历史归档', pinyin: 'lsgd lishi guidang bbc', cat: '功能', ico: '📚', run: () => { closeLauncher(); openMainWindow('notes'); } },
    { name: 'Inkling · 待办清单', pinyin: 'dbqd daiban todo', cat: '功能', ico: '✅', run: () => { closeLauncher(); openMainWindow('todos'); } },
    { name: 'Inkling · 剪贴板历史', pinyin: 'jtb jiantieban clip', cat: '功能', ico: '📋', run: () => { closeLauncher(); openMainWindow('clips'); } },
    { name: 'Inkling · 新建思维导图', pinyin: 'xjsdt xinjian siweidaotu mindmap', cat: '功能', ico: '🧠', run: () => { closeLauncher(); openMindmapEditor(null); } },
    { name: 'Inkling · 统计报表', pinyin: 'tjbb tongji stats', cat: '功能', ico: '📊', run: () => { closeLauncher(); openMainWindow('stats'); } },
    { name: 'Inkling · 偏好设置', pinyin: 'phsz pianhao shezhi config', cat: '功能', ico: '⚙️', run: () => { closeLauncher(); openMainWindow('settings'); } },
  ];

  let launcherIndex = 0;
  let launcherResults = [];

  function openLauncher() {
    hidePanel(); closeAllWindows();
    const win = $('launcherWindow');
    win.classList.remove('hidden');
    $('launcherInput').value = '';
    launcherIndex = 0;
    renderLauncherResults('');
    setTimeout(() => $('launcherInput').focus(), 30);
    gsap.fromTo(win, { y: -18, opacity: 0, scale: .98 }, { y: 0, opacity: 1, scale: 1, duration: .2, ease: 'power2.out' });
  }

  function closeLauncher() {
    $('launcherWindow').classList.add('hidden');
    $('launcherInput').blur();
  }

  function renderLauncherResults(q) {
    const raw = (q || '').trim();
    const query = raw.toLowerCase();
    launcherResults = [];

    // 计算器支持：以 = 开头或纯数学表达式
    if (raw.startsWith('=') || (/^[\d\s+\-*/().%^]+$/.test(raw) && /[+\-*/%]/.test(raw))) {
      const expr = raw.startsWith('=') ? raw.slice(1).trim() : raw;
      try {
        // 安全简易求值（仅允许数字与基本运算符）
        if (/^[0-9+\-*/().\s%]+$/.test(expr)) {
          const res = Function(`"use strict"; return (${expr})`)();
          if (typeof res === 'number' && !isNaN(res)) {
            launcherResults.push({
              ico: '🧮', name: `${expr} = ${res}`, sub: '回车复制结果到剪贴板', cat: '计算器',
              run: () => { addClip(String(res), 'text'); toast(`已复制结果：${res}`); closeLauncher(); }
            });
          }
        }
      } catch (_) {}
    }

    // 匹配应用与内置命令
    LAUNCHER_APPS.forEach(app => {
      if (!query || app.name.toLowerCase().includes(query) || app.pinyin.includes(query)) {
        launcherResults.push({ ico: app.ico, name: app.name, sub: '', cat: app.cat, run: app.run });
      }
    });

    // 搜索笔记
    if (query) {
      notes.forEach(n => {
        const text = n.text.toLowerCase();
        const tags = (n.tags || []).join(' ').toLowerCase();
        if (text.includes(query) || tags.includes(query) || (n.type === 'mindmap' && mindmapAllText(n.mindmapData).includes(query))) {
          launcherResults.push({
            ico: n.type === 'mindmap' ? '🧠' : '📝',
            name: (n.type === 'mindmap' ? '思维导图: ' : '笔记: ') + n.text.slice(0, 36),
            sub: n.tags?.length ? `[${n.tags.join(', ')}]` : n.time,
            cat: '笔记',
            run: () => {
              closeLauncher();
              if (n.type === 'mindmap') openMindmapEditor(n.id);
              else openNoteInPanel(n.id);
            }
          });
        }
      });

      // 搜索待办
      todos.forEach(t => {
        if (t.text.toLowerCase().includes(query)) {
          launcherResults.push({
            ico: '✅', name: `待办: ${t.text}`, sub: `${t.date} · 优先级 ${t.priority}`, cat: '待办',
            run: () => { closeLauncher(); openMainWindow('todos'); }
          });
        }
      });
    }

    if (launcherIndex >= launcherResults.length) launcherIndex = Math.max(0, launcherResults.length - 1);

    if (!launcherResults.length) {
      $('launcherList').innerHTML = `<div class="launcher-empty">未找到匹配项，按 Enter 用默认浏览器搜索「${escapeHtml(raw)}」</div>`;
      $('launcherFooter').innerHTML = `<span>↵ 回车搜索 · Esc 退出</span>`;
      return;
    }

    $('launcherList').innerHTML = launcherResults.map((r, i) => `
      <div class="launcher-item ${i === launcherIndex ? 'active' : ''}" data-idx="${i}">
        <span class="li-ico">${r.ico}</span>
        <span class="li-name">${escapeHtml(r.name)}${r.sub ? `<small>${escapeHtml(r.sub)}</small>` : ''}</span>
        <span class="li-cat">${r.cat}</span>
      </div>`).join('');

    $('launcherFooter').innerHTML = `<span>↑↓ 导航 · ↵ 执行 · Esc 关闭 · 共 ${launcherResults.length} 项</span>`;

    // 鼠标点击执行
    document.querySelectorAll('.launcher-item').forEach(el => {
      el.addEventListener('click', () => {
        const idx = Number(el.dataset.idx);
        if (launcherResults[idx]) launcherResults[idx].run();
      });
      el.addEventListener('mouseenter', () => {
        launcherIndex = Number(el.dataset.idx);
        document.querySelectorAll('.launcher-item').forEach((x, i) => x.classList.toggle('active', i === launcherIndex));
      });
    });
  }

  // 启动台输入与键盘导航
  $('launcherInput').addEventListener('input', (e) => {
    launcherIndex = 0;
    renderLauncherResults(e.target.value);
  });
  $('launcherInput').addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (!launcherResults.length) return;
      launcherIndex = (launcherIndex + 1) % launcherResults.length;
      document.querySelectorAll('.launcher-item').forEach((x, i) => x.classList.toggle('active', i === launcherIndex));
      document.querySelectorAll('.launcher-item')[launcherIndex]?.scrollIntoView({ block: 'nearest' });
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (!launcherResults.length) return;
      launcherIndex = (launcherIndex - 1 + launcherResults.length) % launcherResults.length;
      document.querySelectorAll('.launcher-item').forEach((x, i) => x.classList.toggle('active', i === launcherIndex));
      document.querySelectorAll('.launcher-item')[launcherIndex]?.scrollIntoView({ block: 'nearest' });
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (launcherResults[launcherIndex]) {
        launcherResults[launcherIndex].run();
      } else {
        const q = $('launcherInput').value.trim();
        if (q) {
          window.open(`https://www.google.com/search?q=${encodeURIComponent(q)}`, '_blank', 'noopener,noreferrer');
          closeLauncher();
        }
      }
    }
  });

  // 点击启动台外部关闭
  document.addEventListener('mousedown', (e) => {
    if (!$('launcherWindow').classList.contains('hidden') && !e.target.closest('#launcherWindow,#demoLauncher')) {
      closeLauncher();
    }
  });

  // ── 演示控制条 ────────────────────────────────
  $('demoTogglePanel').addEventListener('click', () => panelVisible ? hidePanel() : showPanel());
  $('demoLauncher').addEventListener('click', () => $('launcherWindow').classList.contains('hidden') ? openLauncher() : closeLauncher());
  $('demoReminder').addEventListener('click', () => showReminder('记得给产品文档补充截图'));
  $('demoReset').addEventListener('click', () => location.reload());

  // ── 引导层 ────────────────────────────────────
  $('onboardDismiss').addEventListener('click', () => {
    gsap.to($('onboarding'), { opacity: 0, scale: .95, duration: .25, onComplete: () => $('onboarding').classList.add('hidden') });
    setTimeout(showPanel, 400);
  });

  // 时钟
  setInterval(() => { $('clock').textContent = new Date().toTimeString().slice(0,5); }, 1000);

  // 键盘可达性：以 role="button" / "tab" / "menuitem" 承担按钮职责的非 button 元素，
  // 原生不响应键盘。此处统一委托 Enter / Space 触发点击，避免逐个元素重复绑定（需求 3.2）。
  document.addEventListener('keydown', (e) => {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    const el = e.target;
    if (!(el instanceof HTMLElement)) return;
    if (el.tagName === 'BUTTON' || el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') return;
    if (el.isContentEditable) return;
    const role = el.getAttribute('role');
    if (role !== 'button' && role !== 'tab' && role !== 'menuitem') return;
    e.preventDefault();      // 阻止 Space 滚动页面
    el.click();
  });

  // 初始渲染
  applyPrefs();              // 先恢复已保存的偏好，再渲染，避免首帧闪现默认值
  buildThemePicker();
  renderClips(); renderTodos(); renderPanelTags();
})();
