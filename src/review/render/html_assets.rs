pub(super) const STYLE: &str = r#"
  :root { color-scheme: light; }
  body { font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; color: #18202f; background: #f6f7f9; }
  main { max-width: 1180px; margin: 0 auto; padding: 28px; }
  header { margin-bottom: 24px; }
  h1 { font-size: 1.75rem; margin: 0 0 .35rem; }
  h2 { font-size: 1.08rem; margin: 28px 0 12px; }
  h3 { font-size: .96rem; margin: 18px 0 10px; }
  .meta { color: #5f6b7a; font-size: .9rem; margin: .25rem 0; }
  .proof-card, .panel { background: #fff; border: 1px solid #dde2ea; border-radius: 8px; padding: 16px 18px; margin-bottom: 14px; }
  .proof-card { border-left: 5px solid #667085; }
  .proof-card.verdict-broken { border-left-color: #b42318; }
  .proof-card.verdict-review { border-left-color: #b54708; }
  .proof-card.verdict-verified { border-left-color: #067647; }
  .proof-card.verdict-not-assessed { border-left-color: #667085; }
  .proof-header { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .proof-header h2 { margin: 0; }
  .badge { display: inline-block; padding: .15rem .5rem; border-radius: 999px; font-size: .72rem; font-weight: 700; text-transform: uppercase; }
  .badge.broken, .badge.blocked { background: #fef3f2; color: #b42318; }
  .badge.review { background: #fffaeb; color: #b54708; }
  .badge.verified, .badge.ready { background: #ecfdf3; color: #067647; }
  .badge.not-assessed, .badge.disabled { background: #eef1f5; color: #475467; }
  .proof-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 10px; margin: 16px 0; }
  .metric { background: #f8fafc; border: 1px solid #edf0f4; border-radius: 6px; padding: 10px 12px; }
  .metric dt { color: #667085; font-size: .72rem; text-transform: uppercase; letter-spacing: .04em; }
  .metric dd { margin: 4px 0 0; font-size: .92rem; }
  .next-action { background: #f0f7ff; border: 1px solid #cfe5ff; border-radius: 6px; padding: 10px 12px; }
  .next-action strong { color: #155eef; }
  .limits, .reasons { margin: 8px 0 0; padding-left: 20px; }
  table { width: 100%; border-collapse: collapse; background: #fff; border: 1px solid #dde2ea; border-radius: 8px; overflow: hidden; }
  th { text-align: left; padding: .6rem .75rem; background: #eef1f5; color: #475467; font-size: .74rem; text-transform: uppercase; letter-spacing: .04em; }
  td { padding: .62rem .75rem; border-top: 1px solid #edf0f4; vertical-align: top; font-size: .86rem; }
  details { margin: 0; }
  summary { color: #155eef; cursor: pointer; }
  .diff-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; margin-top: 8px; }
  .diff-side { min-width: 0; }
  .diff-side strong { color: #475467; font-size: .76rem; text-transform: uppercase; }
  pre { margin: 4px 0 0; padding: 8px; background: #f3f5f7; border-radius: 5px; overflow: auto; white-space: pre-wrap; }
  .status { font-size: .72rem; font-weight: 700; text-transform: uppercase; }
  .status.added, .status.modified, .status.renamed, .status.untracked { color: #067647; }
  .status.deleted { color: #b42318; }
  .signal { border: 1px solid #dde2ea; border-radius: 6px; padding: 10px 12px; margin: 8px 0; background: #fff; }
  .signal strong { font-size: .9rem; }
  .signal-meta, .muted { color: #667085; font-size: .82rem; }
  .empty { color: #667085; font-style: italic; }
  code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .9em; }
  a { color: #155eef; }
  @media (max-width: 680px) { main { padding: 16px; } table { display: block; overflow-x: auto; } .diff-grid { grid-template-columns: 1fr; } }
"#;

pub(super) const SCRIPT: &str = r#"
  document.querySelectorAll('[data-jump]').forEach(link => {
    link.addEventListener('click', event => {
      const target = document.getElementById(link.dataset.jump);
      if (target) { event.preventDefault(); target.scrollIntoView({ behavior: 'smooth' }); }
    });
  });
"#;
