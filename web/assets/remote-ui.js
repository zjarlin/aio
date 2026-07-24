const catalogElement = document.querySelector('#remote-ui-catalog');
const root = document.querySelector('#remote-ui-root');
const replayButton = document.querySelector('#remote-ui-replay');
const statusElement = document.querySelector('#remote-ui-status');
const eventLog = document.querySelector('#remote-ui-event-log');
const eventCount = document.querySelector('#remote-ui-event-count');

if (!catalogElement || !root || !replayButton || !statusElement || !eventLog || !eventCount) {
  throw new Error('Remote UI bootstrap elements are incomplete');
}

const catalog = JSON.parse(catalogElement.textContent || '{}');
const nodesById = new Map();
let mountStack = [root];
let kindStack = [];
let source = null;
let receivedEvents = 0;

function setStatus(label, state) {
  statusElement.textContent = label;
  statusElement.className = `remote-ui-status remote-ui-status--${state}`;
}

function resetRenderer() {
  if (source) {
    source.close();
    source = null;
  }
  root.replaceChildren();
  eventLog.replaceChildren();
  nodesById.clear();
  mountStack = [root];
  kindStack = [];
  receivedEvents = 0;
  eventCount.textContent = '0';
}

function connect() {
  resetRenderer();
  setStatus('流式渲染', 'streaming');
  source = new EventSource(root.dataset.streamUrl);
  source.onmessage = (event) => {
    applyOperation(JSON.parse(event.data));
  };
  source.addEventListener('done', () => {
    source?.close();
    source = null;
    setStatus('已完成', 'ready');
  });
  source.onerror = () => {
    source?.close();
    source = null;
    setStatus('连接失败', 'error');
  };
}

function applyOperation(operation) {
  switch (operation.op) {
    case 'open':
      openNode(operation.node);
      break;
    case 'leaf':
      appendNode(operation.node);
      break;
    case 'text':
      currentMount().append(document.createTextNode(operation.value));
      break;
    case 'close':
      closeNode(operation.kind);
      break;
    case 'patch':
      patchNode(operation.id, operation.attributes);
      break;
    default:
      recordEvent('协议错误', { operation });
  }
}

function openNode(node) {
  const rendered = renderNode(node);
  currentMount().append(rendered.outer);
  mountStack.push(rendered.mount);
  kindStack.push(node.kind);
}

function appendNode(node) {
  const rendered = renderNode(node);
  currentMount().append(rendered.outer);
}

function closeNode(kind) {
  const openKind = kindStack.pop();
  if (openKind !== kind || mountStack.length === 1) {
    recordEvent('闭合错误', { expected: openKind, actual: kind });
    return;
  }
  mountStack.pop();
}

function renderNode(node) {
  node.attributes = node.attributes || {};
  const definition = catalog[node.kind];
  if (!definition) {
    return renderUnsupported(node);
  }
  const spec = definition.spec;
  if (spec.behavior === 'input') {
    return renderInput(node, spec);
  }
  if (spec.behavior === 'progress') {
    return renderProgress(node, spec);
  }

  const element = document.createElement(spec.html_tag);
  applySpec(element, node, spec);
  if (spec.behavior === 'table') {
    const wrapper = document.createElement('div');
    wrapper.className = 'remote-ui-table-wrapper relative w-full overflow-x-auto rounded-xl border';
    wrapper.append(element);
    rememberNode(node, element);
    return { outer: wrapper, mount: element };
  }
  if (node.content) {
    element.textContent = node.content;
  } else if (node.attributes.tx) {
    element.textContent = node.attributes.tx;
  }
  if (spec.behavior === 'button') {
    bindAction(element, node.attributes.act);
  }
  rememberNode(node, element);
  return { outer: element, mount: element };
}

function applySpec(element, node, spec) {
  const variant = node.attributes.v || spec.default_variant || '';
  const variantClass = spec.variants?.[variant] || '';
  element.className = `${spec.class_name} ${variantClass}`.trim();
  if (variant) {
    element.dataset.variant = variant;
  }
  if (node.id) {
    element.id = node.id;
  }
  for (const name of ['disabled', 'required', 'checked', 'multiple', 'readonly']) {
    if (node.attributes[name] === 'true') {
      element[name] = true;
    }
  }
}

function renderInput(node, spec) {
  const field = document.createElement('label');
  field.className = 'remote-ui-field';
  const label = document.createElement('span');
  label.className = 'remote-ui-field-label';
  label.textContent = node.attributes.label || '';
  const input = document.createElement('input');
  applySpec(input, node, spec);
  input.type = node.attributes.type || 'text';
  input.placeholder = node.attributes.ph || '';
  input.name = node.attributes.name || node.id || '';
  input.addEventListener('change', () => {
    recordEvent('input.change', { id: node.id, value: input.value });
  });
  field.append(label, input);
  rememberNode(node, input);
  return { outer: field, mount: field };
}

function renderProgress(node, spec) {
  const progress = document.createElement('div');
  applySpec(progress, node, spec);
  const header = document.createElement('div');
  header.className = 'remote-ui-progress-header';
  const label = document.createElement('span');
  label.textContent = node.attributes.label || '进度';
  const value = document.createElement('span');
  value.dataset.progressValue = 'true';
  const track = document.createElement('div');
  track.className = 'remote-ui-progress-track';
  const bar = document.createElement('div');
  bar.className = 'remote-ui-progress-bar';
  bar.dataset.progressBar = 'true';
  track.append(bar);
  header.append(label, value);
  progress.append(header, track);
  rememberNode(node, progress);
  updateProgress(progress, node.attributes);
  return { outer: progress, mount: progress };
}

function renderUnsupported(node) {
  const element = document.createElement('div');
  element.className = 'rounded-md border border-destructive bg-destructive/10 p-4 text-sm text-destructive';
  element.textContent = `不支持的组件: ${node.kind}`;
  return { outer: element, mount: element };
}

function patchNode(id, attributes) {
  const element = nodesById.get(id);
  if (!element) {
    recordEvent('更新目标不存在', { id, attributes });
    return;
  }
  if (element.classList.contains('remote-ui-progress')) {
    updateProgress(element, attributes);
  }
  if ('ph' in attributes && element instanceof HTMLInputElement) {
    element.placeholder = attributes.ph;
  }
  if ('tx' in attributes) {
    element.textContent = attributes.tx;
  }
}

function updateProgress(element, attributes) {
  const numericValue = Number.parseFloat(attributes.v || '0');
  const value = Number.isFinite(numericValue) ? Math.min(100, Math.max(0, numericValue)) : 0;
  const valueLabel = element.querySelector('[data-progress-value]');
  const bar = element.querySelector('[data-progress-bar]');
  if (valueLabel) valueLabel.textContent = `${value}%`;
  if (bar) bar.style.width = `${value}%`;
  element.dataset.status = attributes.status || element.dataset.status || 'running';
  element.setAttribute('role', 'progressbar');
  element.setAttribute('aria-valuemin', '0');
  element.setAttribute('aria-valuemax', '100');
  element.setAttribute('aria-valuenow', String(value));
}

function bindAction(element, action) {
  if (!action) return;
  element.dataset.action = action;
  element.addEventListener('click', () => {
    recordEvent('button.click', { action });
  });
}

function rememberNode(node, element) {
  if (node.id) {
    nodesById.set(node.id, element);
  }
}

function currentMount() {
  return mountStack[mountStack.length - 1];
}

function recordEvent(type, detail) {
  receivedEvents += 1;
  eventCount.textContent = String(receivedEvents);
  const item = document.createElement('li');
  const title = document.createElement('strong');
  title.textContent = type;
  const payload = document.createElement('code');
  payload.textContent = JSON.stringify(detail);
  item.append(title, payload);
  eventLog.prepend(item);
}

replayButton.addEventListener('click', connect);
connect();
