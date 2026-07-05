(function () {
  var el = document.getElementById('site-character');
  if (!el) return;

  var currentState = 'idle';
  var timer = null;
  var blinkTimer = null;
  var mouseX = 0;
  var mouseY = 0;
  var hasMouse = false;
  var rafId = null;

  var states = {
    idle:      { weight: 35, min: 2000, max: 4000 },
    smile:     { weight: 20, min: 2500, max: 4000 },
    sleepy:    { weight: 12, min: 3000, max: 4500 },
    sleep:     { weight: 6,  min: 4500, max: 6500 },
    wink:      { weight: 10, min: 250,  max: 350 },
    open:      { weight: 5,  min: 600,  max: 1000 },
  };

  var stateList = [];
  for (var key in states) {
    for (var i = 0; i < states[key].weight; i++) {
      stateList.push(key);
    }
  }

  var pupilTargets = { x: 0, y: 0 };
  var pupilCurrent = { x: 0, y: 0 };
  var eyesVisible = true;

  function isEyesVisible(state) {
    return state !== 'sleep' && state !== 'sleepy' && state !== 'smile';
  }

  function applyState(state) {
    el.className = el.className.replace(/\bstate-\S+/g, '').trim();
    if (state !== 'idle') {
      el.classList.add('state-' + state);
    }
    if (state === 'wink') {
      el.classList.add('state-wink');
    }
    currentState = state;
    eyesVisible = isEyesVisible(state);
    if (!eyesVisible) {
      pupilTargets.x = 0;
      pupilTargets.y = 0;
    }
  }

  function randomInt(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
  }

  function pickState() {
    return stateList[Math.floor(Math.random() * stateList.length)];
  }

  function transitionTo(state) {
    clearTimeout(timer);
    clearTimeout(blinkTimer);
    applyState(state);
    var cfg = states[state];
    if (!cfg) return;
    var duration = randomInt(cfg.min, cfg.max);
    timer = setTimeout(function () {
      goIdle();
    }, duration);
  }

  function goIdle() {
    applyState('idle');
    scheduleBlink();
    timer = setTimeout(function () {
      var next = pickState();
      if (next === 'idle') {
        goIdle();
      } else {
        transitionTo(next);
      }
    }, randomInt(3000, 7000));
  }

  function blink() {
    if (currentState !== 'idle') return;
    el.classList.add('state-blink');
    setTimeout(function () {
      el.classList.remove('state-blink');
    }, 120);
    scheduleBlink();
  }

  function scheduleBlink() {
    clearTimeout(blinkTimer);
    blinkTimer = setTimeout(blink, randomInt(2500, 5000));
  }

  // Mouse tracking
  document.addEventListener('mousemove', function (e) {
    mouseX = e.clientX;
    mouseY = e.clientY;
    hasMouse = true;
  });

  function trackPupils() {
    if (!hasMouse || !eyesVisible) {
      pupilCurrent.x += (pupilTargets.x - pupilCurrent.x) * 0.08;
      pupilCurrent.y += (pupilTargets.y - pupilCurrent.y) * 0.08;
    } else {
      var rect = el.getBoundingClientRect();
      var cx = rect.left + rect.width / 2;
      var cy = rect.top + rect.height / 2;
      var dx = mouseX - cx;
      var dy = mouseY - cy;
      var dist = Math.sqrt(dx * dx + dy * dy);
      var maxDist = Math.max(rect.width, rect.height) * 1.5;
      var clamp = Math.min(dist / maxDist, 1);
      var maxPx = 8;
      var tx = (dx / dist) * clamp * maxPx;
      var ty = (dy / dist) * clamp * maxPx;
      if (dist < 5) { tx = 0; ty = 0; }
      pupilTargets.x = tx;
      pupilTargets.y = ty;
    }

    pupilCurrent.x += (pupilTargets.x - pupilCurrent.x) * 0.12;
    pupilCurrent.y += (pupilTargets.y - pupilCurrent.y) * 0.12;

    if (Math.abs(pupilCurrent.x) > 0.5 || Math.abs(pupilCurrent.y) > 0.5) {
      el.style.setProperty('--pupil-x', pupilCurrent.x + 'px');
      el.style.setProperty('--pupil-y', pupilCurrent.y + 'px');
    }

    rafId = requestAnimationFrame(trackPupils);
  }

  // Reset pupils when mouse leaves
  document.addEventListener('mouseleave', function () {
    hasMouse = false;
  });

  trackPupils();
  goIdle();
})();