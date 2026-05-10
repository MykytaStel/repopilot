pub(super) const STYLE: &str = r#"
  :root { color-scheme: light; }
  body { font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; color: #18202f; background: #f6f7f9; }
  main { max-width: 1180px; margin: 0 auto; padding: 28px; }
  header { margin-bottom: 24px; }
  h1 { font-size: 1.75rem; margin: 0 0 0.35rem; letter-spacing: 0; }
  h2 { font-size: 1.05rem; margin: 28px 0 12px; }
  h3 { font-size: 0.98rem; margin: 18px 0 10px; }
  h4 { font-size: 0.92rem; margin: 14px 0 8px; }
  .meta { color: #5f6b7a; font-size: 0.9rem; margin: 0.25rem 0; }
  .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 10px; margin: 18px 0 20px; }
  .card { background: #fff; border: 1px solid #dde2ea; border-radius: 8px; padding: 14px 16px; }
  .card .num { font-size: 1.45rem; line-height: 1.1; font-weight: 750; }
  .card .label { margin-top: 5px; color: #667085; font-size: 0.74rem; text-transform: uppercase; letter-spacing: .04em; }
  .panel { background: #fff; border: 1px solid #dde2ea; border-radius: 8px; padding: 14px 16px; margin-bottom: 12px; }
  .inline-list { display: flex; flex-wrap: wrap; gap: 8px; margin: 0; padding: 0; list-style: none; }
  .pill { display: inline-flex; align-items: center; gap: 5px; border: 1px solid #d5dbe5; border-radius: 999px; padding: 3px 9px; font-size: 0.82rem; background: #fff; }
  .badge { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 999px; font-size: 0.72rem; font-weight: 700; text-transform: uppercase; }
  .badge.info { background: #e8f4ff; color: #155eef; }
  .badge.low { background: #ecfdf3; color: #067647; }
  .badge.medium { background: #fffaeb; color: #b54708; }
  .badge.high { background: #fff4ed; color: #c4320a; }
  .badge.critical { background: #fef3f2; color: #b42318; }
  .status { font-size: 0.72rem; font-weight: 700; text-transform: uppercase; }
  .status.new { color: #b42318; }
  .status.existing { color: #067647; }
  table { width: 100%; border-collapse: collapse; background: #fff; border: 1px solid #dde2ea; border-radius: 8px; overflow: hidden; }
  th { text-align: left; padding: 0.6rem 0.75rem; background: #eef1f5; color: #475467; font-size: 0.76rem; text-transform: uppercase; letter-spacing: .04em; }
  td { padding: 0.62rem 0.75rem; border-top: 1px solid #edf0f4; vertical-align: top; font-size: 0.88rem; }
  .num-cell { text-align: right; }
  .filters { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 14px; }
  .filter-chip { border: 1px solid #cfd6e0; background: #fff; border-radius: 999px; color: #2d3748; cursor: pointer; font: inherit; font-size: 0.82rem; padding: 5px 10px; }
  .filter-chip.active { background: #1f2937; border-color: #1f2937; color: #fff; }
  .filter-chip.clear { color: #475467; }
  .finding-group { margin-bottom: 20px; }
  .rule-group { margin: 12px 0 18px; }
  .finding-card { background: #fff; border: 1px solid #dde2ea; border-radius: 8px; margin: 8px 0; padding: 12px 14px; }
  .finding-title { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }
  .finding-title strong { font-size: 0.95rem; }
  .finding-meta { color: #5f6b7a; font-size: 0.84rem; margin: 4px 0; }
  pre.snippet { margin: 8px 0 0; font-size: 0.8rem; background: #f3f5f7; padding: 8px 10px; border-radius: 6px; overflow: auto; white-space: pre-wrap; }
  .empty { color: #667085; font-style: italic; }
  code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.9em; }
"#;

pub(super) const SCRIPT: &str = r#"
  const filters = {
    severity: new Set(),
    category: new Set(),
    rule: new Set(),
  };

  function matchesFilters(card) {
    return Object.entries(filters).every(([type, selected]) => {
      return selected.size === 0 || selected.has(card.dataset[type]);
    });
  }

  function refreshFindings() {
    document.querySelectorAll('.finding-card').forEach(card => {
      card.hidden = !matchesFilters(card);
    });
    document.querySelectorAll('.finding-group, .rule-group').forEach(group => {
      const cards = [...group.querySelectorAll('.finding-card')];
      group.hidden = cards.length > 0 && cards.every(card => card.hidden);
    });
  }

  document.querySelectorAll('[data-filter-type]').forEach(button => {
    button.addEventListener('click', () => {
      const type = button.dataset.filterType;
      const value = button.dataset.filterValue;
      if (filters[type].has(value)) {
        filters[type].delete(value);
        button.classList.remove('active');
      } else {
        filters[type].add(value);
        button.classList.add('active');
      }
      refreshFindings();
    });
  });

  document.querySelector('[data-filter-clear]')?.addEventListener('click', () => {
    Object.values(filters).forEach(set => set.clear());
    document.querySelectorAll('.filter-chip.active').forEach(button => button.classList.remove('active'));
    refreshFindings();
  });
"#;
