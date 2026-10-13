(() => {
  const tables = [...document.querySelectorAll('.table-wrapper')];
  if (!tables.length) return;

  const scrollbar = document.createElement('input');
  scrollbar.type = 'range';
  scrollbar.min = '0';
  scrollbar.step = '1';
  scrollbar.className = 'table-scrollbar';
  scrollbar.hidden = true;
  scrollbar.setAttribute('aria-label', 'Scroll visible table horizontally');
  document.body.appendChild(scrollbar);
  let activeTable = null;
  let pending = false;

  function update() {
    pending = false;
    let visibleHeight = 0;
    activeTable = null;
    for (const table of tables) {
      const rect = table.getBoundingClientRect();
      const height = Math.min(rect.bottom, innerHeight - 24) - Math.max(rect.top, 0);
      if (table.scrollWidth > table.clientWidth && height > visibleHeight) {
        activeTable = table;
        visibleHeight = height;
      }
    }
    scrollbar.hidden = !activeTable;
    if (!activeTable) return;

    const rect = activeTable.getBoundingClientRect();
    scrollbar.style.left = `${rect.left}px`;
    scrollbar.style.width = `${activeTable.clientWidth}px`;
    scrollbar.max = activeTable.scrollWidth - activeTable.clientWidth;
    scrollbar.value = activeTable.scrollLeft;
    scrollbar.style.setProperty('--thumb-width', `${Math.max(24, activeTable.clientWidth ** 2 / activeTable.scrollWidth)}px`);
  }

  function scheduleUpdate() {
    if (!pending) {
      pending = true;
      requestAnimationFrame(update);
    }
  }

  scrollbar.addEventListener('input', () => {
    if (activeTable) activeTable.scrollLeft = Number(scrollbar.value);
  });
  // Keep mdBook's chapter shortcuts from capturing scrollbar arrow keys.
  scrollbar.addEventListener('keydown', event => event.stopPropagation());
  document.addEventListener('scroll', scheduleUpdate, { capture: true, passive: true });
  window.addEventListener('resize', scheduleUpdate);
  document.addEventListener('transitionend', scheduleUpdate);
  const observer = new ResizeObserver(scheduleUpdate);
  for (const table of tables) {
    observer.observe(table);
    observer.observe(table.querySelector('table'));
  }
  update();
})();
