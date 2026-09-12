(() => {
  const failed = () => {
    const status = document.getElementById('startup');
    if (status) {
      status.setAttribute('role', 'alert');
      status.textContent = '加载失败，请重新打开页面。';
    }
  };
  window.addEventListener('error', failed, { capture: true });
  window.addEventListener('unhandledrejection', failed);
})();
