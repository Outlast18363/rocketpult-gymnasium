// The Rust game calls this tiny bridge to persist only its high score.
miniquad_add_plugin({
  register_plugin: function (imports) {
    imports.env.rocketpult_load_best = function () {
      try { return Math.max(0, Number(localStorage.getItem('rocketpult_rust_best')) || 0) >>> 0; }
      catch (_) { return 0; }
    };
    imports.env.rocketpult_save_best = function (score) {
      try { localStorage.setItem('rocketpult_rust_best', String(score >>> 0)); }
      catch (_) { /* Flight continues if the browser disables persistent storage. */ }
    };
  },
  name: 'rocketpult_storage',
  version: 1
});
