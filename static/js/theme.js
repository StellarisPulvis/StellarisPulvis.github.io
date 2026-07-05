(function () {
  /* ---------- Theme ---------- */
  var STORAGE_KEY = 'stellaris-theme';
  var html = document.documentElement;

  function getSystemTheme() {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function getStoredTheme() {
    return localStorage.getItem(STORAGE_KEY);
  }

  function setTheme(theme) {
    if (theme === 'dark' || theme === 'light') {
      html.setAttribute('data-theme', theme);
      localStorage.setItem(STORAGE_KEY, theme);
    } else {
      html.removeAttribute('data-theme');
      localStorage.removeItem(STORAGE_KEY);
    }
    updateToggleIcon();
    updateParticles();
  }

  function getEffectiveTheme() {
    return getStoredTheme() || getSystemTheme();
  }

  /* ---------- Toggle Button ---------- */
  var btn = document.getElementById('theme-toggle');
  if (btn) {
    btn.addEventListener('click', function () {
      var current = getEffectiveTheme();
      setTheme(current === 'dark' ? 'light' : 'dark');
    });
  }

  function updateToggleIcon() {
    if (!btn) return;
    var theme = getEffectiveTheme();
    btn.textContent = theme === 'dark' ? '\u2600\uFE0F' : '\uD83C\uDF19';
  }

  /* ---------- Init theme on load ---------- */
  var stored = getStoredTheme();
  if (stored) {
    html.setAttribute('data-theme', stored);
  } else {
    // respect system preference
    var mq = window.matchMedia('(prefers-color-scheme: dark)');
    if (mq.matches) html.setAttribute('data-theme', 'dark');
  }
  updateToggleIcon();

  // listen for system theme changes when user hasn't set a preference
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function () {
    if (!getStoredTheme()) {
      setTheme(getSystemTheme());
    }
  });

  /* ---------- Stardust Particles ---------- */
  var canvas, ctx, particles, animId;
  var PARTICLE_COUNT = 60;

  function initCanvas() {
    canvas = document.getElementById('stardust-canvas');
    if (!canvas) return;
    ctx = canvas.getContext('2d');
    resize();
    window.addEventListener('resize', resize);
    particles = [];
    for (var i = 0; i < PARTICLE_COUNT; i++) {
      particles.push(createParticle(true));
    }
    animate();
  }

  function resize() {
    if (!canvas) return;
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
  }

  function createParticle(randomY) {
    return {
      x: Math.random() * canvas.width,
      y: randomY ? Math.random() * canvas.height : canvas.height + 10,
      r: Math.random() * 2 + 0.5,
      speed: Math.random() * 0.3 + 0.05,
      drift: (Math.random() - 0.5) * 0.2,
      opacity: Math.random() * 0.5 + 0.1,
      twinkleSpeed: Math.random() * 0.02 + 0.005,
      twinklePhase: Math.random() * Math.PI * 2,
    };
  }

  function animate() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    for (var i = 0; i < particles.length; i++) {
      var p = particles[i];
      p.y -= p.speed;
      p.x += p.drift + Math.sin(p.y * 0.01) * 0.1;
      // twinkle
      var alpha = p.opacity * (0.6 + 0.4 * Math.sin(p.twinklePhase));
      ctx.beginPath();
      ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(212, 163, 115, ' + alpha + ')';
      ctx.fill();
      p.twinklePhase += p.twinkleSpeed;
      // recycle
      if (p.y < -10) {
        particles[i] = createParticle(false);
      }
      // wrap x
      if (p.x < -10) p.x = canvas.width + 10;
      if (p.x > canvas.width + 10) p.x = -10;
    }
    animId = requestAnimationFrame(animate);
  }

  function updateParticles() {
    var theme = getEffectiveTheme();
    if (canvas) {
      canvas.style.display = theme === 'dark' ? 'block' : 'none';
    }
  }

  // start
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initCanvas);
  } else {
    initCanvas();
  }
})();
