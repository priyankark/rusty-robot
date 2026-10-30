'use strict';
for (const button of document.querySelectorAll('[data-video][data-rate]')) {
  button.addEventListener('click', async () => {
    const video = document.getElementById(button.dataset.video);
    const status = document.getElementById(`${video.id}-status`);
    const rate = Number(button.dataset.rate);
    video.playbackRate = rate;
    video.currentTime = 0;
    status.textContent = rate === 1
      ? 'Playback: original speed (1×). Separate recorded takes; not the measured medians.'
      : 'Playback: quarter speed (¼×), slowed 4× for inspection. Separate recorded takes.';
    try { await video.play(); }
    catch { status.textContent += ' Use the video play control to start.'; }
  });
}
