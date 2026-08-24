(() => {
  const MAX_COMMENTS = 3;
  const HEARTBEAT_TIMEOUT_MS = 30_000;
  const HEARTBEAT_CHECK_MS = 1_000;
  const RECONNECT_BASE_MS = 500;
  const RECONNECT_MAX_MS = 30_000;
  const LEAVE_FALLBACK_MS = 190;
  const HIGHLIGHT_DURATION_MS = 8_000;
  const HIGHLIGHT_LEAVE_MS = 200;
  const EFFECT_DURATION_MS = 2_100;
  const EFFECT_PARTICLE_COUNT = 10;
  const MAX_EFFECT_GROUPS = 4;
  const VALID_THEMES = new Set(["dark", "light", "compact"]);
  const EFFECTS = new Map([
    ["peace", { label: "✌️", emoji: true }],
    ["rock_on", { label: "🤟", emoji: true }],
    ["laugh", { label: "www", emoji: false }],
    ["good_game", { label: "GG", emoji: false }],
    ["grass", { label: "草", emoji: false }],
  ]);
  const DISCORD_CDN_HOST = "cdn.discordapp.com";
  const STATIC_AVATAR_PATH = /^\/(?:avatars\/\d+\/[a-zA-Z0-9_-]+|guilds\/\d+\/users\/\d+\/avatars\/[a-zA-Z0-9_-]+|embed\/avatars\/\d+)\.(?:png|jpe?g|webp)$/;

  const overlay = document.querySelector("#overlay");
  const commentsElement = document.querySelector("#comments");
  const effectsElement = document.querySelector("#effects");
  const highlightElement = document.querySelector("#highlight");
  const commentTemplate = document.querySelector("#comment-template");
  const highlightTemplate = document.querySelector("#highlight-template");
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  if (!(overlay instanceof HTMLElement)
    || !(commentsElement instanceof HTMLOListElement)
    || !(effectsElement instanceof HTMLElement)
    || !(highlightElement instanceof HTMLElement)
    || !(commentTemplate instanceof HTMLTemplateElement)
    || !(highlightTemplate instanceof HTMLTemplateElement)) {
    return;
  }

  const capability = window.location.hash.slice(1);
  const publicId = getPublicId(window.location.pathname);
  const query = new URLSearchParams(window.location.search);
  const queryTheme = query.get("theme");

  let theme = VALID_THEMES.has(queryTheme) ? queryTheme : "dark";
  let showAvatar = query.get("avatars") !== "false" && query.get("avatars") !== "0";
  let socket = null;
  let reconnectTimer = 0;
  let heartbeatTimer = 0;
  let reconnectAttempts = 0;
  let lastHeartbeatAt = 0;
  let currentRevision = -1;
  let snapshotReceived = false;
  let connectionGeneration = 0;
  let terminal = false;
  let paused = false;
  let highlightTimer = 0;
  let highlightLeaveTimer = 0;
  let operationQueue = Promise.resolve();
  const comments = new Map();
  const expiryTimers = new Map();
  const effectTimers = new Map();

  applyAppearance();

  if (!capability || !publicId) {
    makeTransparentImmediately();
    return;
  }

  connect();
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      clearTransientPresentation();
    }
  });

  function getPublicId(pathname) {
    const match = pathname.match(/\/overlay\/([^/]+)\/?$/);
    if (!match) {
      return null;
    }

    try {
      return decodeURIComponent(match[1]);
    } catch {
      return null;
    }
  }

  function connect() {
    if (terminal) {
      return;
    }

    window.clearTimeout(reconnectTimer);
    connectionGeneration += 1;
    currentRevision = -1;
    snapshotReceived = false;
    const generation = connectionGeneration;
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/ws/overlay/${encodeURIComponent(publicId)}`;

    try {
      socket = new WebSocket(wsUrl);
    } catch {
      makeTransparentImmediately();
      scheduleReconnect();
      return;
    }

    socket.addEventListener("open", () => {
      if (generation !== connectionGeneration || socket?.readyState !== WebSocket.OPEN) {
        return;
      }

      lastHeartbeatAt = Date.now();
      socket.send(JSON.stringify({ type: "auth", capability }));
      startHeartbeatWatch(generation);
    });

    socket.addEventListener("message", (event) => {
      if (generation !== connectionGeneration || typeof event.data !== "string") {
        return;
      }

      let message;
      try {
        message = JSON.parse(event.data);
      } catch {
        return;
      }

      if (!isObject(message) || typeof message.type !== "string") {
        return;
      }

      if (message.type === "heartbeat") {
        lastHeartbeatAt = Date.now();
        return;
      }

      operationQueue = operationQueue
        .then(() => {
          if (generation === connectionGeneration) {
            return handleMessage(message);
          }
          return undefined;
        })
        .catch(() => {
          if (generation === connectionGeneration) {
            invalidateConnection();
          }
        });
    });

    socket.addEventListener("close", () => {
      if (generation !== connectionGeneration) {
        return;
      }

      connectionGeneration += 1;
      snapshotReceived = false;
      stopHeartbeatWatch();
      makeTransparentImmediately();
      socket = null;
      scheduleReconnect();
    });

    socket.addEventListener("error", () => {
      if (generation === connectionGeneration) {
        invalidateConnection();
      }
    });
  }

  function startHeartbeatWatch(generation) {
    stopHeartbeatWatch();
    heartbeatTimer = window.setInterval(() => {
      if (generation !== connectionGeneration) {
        stopHeartbeatWatch();
        return;
      }

      if (Date.now() - lastHeartbeatAt >= HEARTBEAT_TIMEOUT_MS) {
        invalidateConnection();
      }
    }, HEARTBEAT_CHECK_MS);
  }

  function stopHeartbeatWatch() {
    window.clearInterval(heartbeatTimer);
    heartbeatTimer = 0;
  }

  function invalidateConnection() {
    const closingSocket = socket;
    connectionGeneration += 1;
    snapshotReceived = false;
    stopHeartbeatWatch();
    makeTransparentImmediately();
    socket = null;
    if (closingSocket && closingSocket.readyState < WebSocket.CLOSING) {
      closingSocket.close();
    }
    scheduleReconnect();
  }

  function scheduleReconnect() {
    if (terminal || reconnectTimer) {
      return;
    }

    const exponentialDelay = Math.min(RECONNECT_MAX_MS, RECONNECT_BASE_MS * (2 ** reconnectAttempts));
    const jitteredDelay = Math.round(exponentialDelay * (0.8 + (Math.random() * 0.4)));
    reconnectAttempts = Math.min(reconnectAttempts + 1, 16);
    reconnectTimer = window.setTimeout(() => {
      reconnectTimer = 0;
      connect();
    }, jitteredDelay);
  }

  async function handleMessage(message) {
    if (terminal) {
      return;
    }

    if (message.type === "snapshot") {
      if (!acceptRevision(message.revision)) {
        return;
      }

      reconnectAttempts = 0;
      snapshotReceived = true;
      applySnapshot(message);
      return;
    }

    if (!snapshotReceived) {
      return;
    }

    if (!acceptRevision(message.revision)) {
      return;
    }

    switch (message.type) {
      case "comment_added":
        if (!paused) {
          await addComment(message.comment);
        }
        break;
      case "comment_updated":
        if (!paused) {
          updateComment(message.comment);
        }
        break;
      case "comment_removed":
        await removeComment(message.discord_message_id);
        break;
      case "comment_highlighted":
        if (!paused) {
          showHighlight(message.highlight);
        }
        break;
      case "effect_triggered":
        if (!paused) {
          showEffect(message.effect);
        }
        break;
      case "cleared":
        await clearComments();
        clearTransientPresentation();
        break;
      case "paused":
        paused = true;
        await clearComments();
        clearTransientPresentation();
        break;
      case "resumed":
        paused = false;
        updateVisibility();
        break;
      case "session_ended":
        endSession();
        break;
      default:
        break;
    }
  }

  function acceptRevision(revision) {
    if (!Number.isSafeInteger(revision) || revision < 0 || revision <= currentRevision) {
      return false;
    }
    currentRevision = revision;
    return true;
  }

  function applySnapshot(message) {
    makeTransparentImmediately();

    if (VALID_THEMES.has(message.theme)) {
      theme = message.theme;
    }
    if (typeof message.show_avatar === "boolean") {
      showAvatar = message.show_avatar;
    }
    applyAppearance();

    if (message.status === "ended") {
      endSession();
      return;
    }

    paused = message.status === "paused";
    if (paused || !Array.isArray(message.comments)) {
      updateVisibility();
      return;
    }

    const snapshotComments = message.comments
      .filter(isValidComment)
      .filter((comment) => !isExpired(comment.expires_at))
      .slice(-MAX_COMMENTS);

    for (const comment of snapshotComments) {
      if (comments.has(comment.discord_message_id)) {
        updateComment(comment);
      } else {
        insertComment(comment);
      }
    }
    updateVisibility();
  }

  async function addComment(comment) {
    if (!isValidComment(comment) || isExpired(comment.expires_at)) {
      return;
    }

    if (comments.has(comment.discord_message_id)) {
      updateComment(comment);
      return;
    }

    while (comments.size >= MAX_COMMENTS) {
      const oldestId = comments.keys().next().value;
      await removeComment(oldestId);
    }

    insertComment(comment);
    updateVisibility();
  }

  function insertComment(comment) {
    const fragment = commentTemplate.content.cloneNode(true);
    const element = fragment.querySelector(".comment");
    if (!(element instanceof HTMLLIElement)) {
      return;
    }

    element.dataset.commentId = comment.discord_message_id;
    renderComment(element, comment);
    comments.set(comment.discord_message_id, { element, comment });
    commentsElement.append(fragment);
    scheduleExpiry(comment);
  }

  function updateComment(comment) {
    if (!isValidComment(comment)) {
      return;
    }

    const entry = comments.get(comment.discord_message_id);
    if (!entry) {
      return;
    }

    if (isExpired(comment.expires_at)) {
      void removeComment(comment.discord_message_id);
      return;
    }

    entry.comment = comment;
    renderComment(entry.element, comment);
    scheduleExpiry(comment);
  }

  function renderComment(element, comment) {
    const author = element.querySelector(".comment__author");
    const body = element.querySelector(".comment__body");
    let image = element.querySelector(".comment__avatar-image");
    const fallback = element.querySelector(".comment__avatar-fallback");

    if (author instanceof HTMLElement) {
      author.textContent = comment.author_name;
    }
    if (body instanceof HTMLElement) {
      body.textContent = comment.body;
    }
    if (fallback instanceof HTMLElement) {
      fallback.textContent = getInitial(comment.author_name);
    }
    if (!(image instanceof HTMLImageElement) && showAvatar) {
      image = document.createElement("img");
      image.className = "comment__avatar-image";
      image.alt = "";
      image.referrerPolicy = "no-referrer";
      image.decoding = "async";
      element.querySelector(".comment__avatar")?.append(image);
    }
    if (image instanceof HTMLImageElement) {
      configureAvatar(image, comment.avatar_url);
    }
  }

  function configureAvatar(image, avatarUrl) {
    image.removeAttribute("src");
    image.onerror = null;

    if (!showAvatar) {
      return;
    }

    const safeUrl = getSafeAvatarUrl(avatarUrl);
    if (!safeUrl) {
      return;
    }

    image.onerror = () => {
      image.onerror = null;
      image.removeAttribute("src");
    };
    image.src = safeUrl;
  }

  function getSafeAvatarUrl(value) {
    if (typeof value !== "string" || value.length > 512) {
      return null;
    }

    try {
      const url = new URL(value);
      if (url.protocol !== "https:" || url.hostname !== DISCORD_CDN_HOST || !STATIC_AVATAR_PATH.test(url.pathname)) {
        return null;
      }
      if ([...url.searchParams.keys()].some((key) => key !== "size")) {
        return null;
      }
      if (url.searchParams.has("size") && !/^\d{2,4}$/.test(url.searchParams.get("size") ?? "")) {
        return null;
      }
      if (url.username || url.password || url.port || url.hash) {
        return null;
      }
      return url.href;
    } catch {
      return null;
    }
  }

  function getInitial(name) {
    const trimmed = name.trim();
    if (!trimmed) {
      return "?";
    }

    if (typeof Intl.Segmenter === "function") {
      const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });
      return segmenter.segment(trimmed)[Symbol.iterator]().next().value?.segment ?? "?";
    }
    return Array.from(trimmed)[0] ?? "?";
  }

  function scheduleExpiry(comment) {
    window.clearTimeout(expiryTimers.get(comment.discord_message_id));
    expiryTimers.delete(comment.discord_message_id);

    const expiresAt = Date.parse(comment.expires_at);
    if (!Number.isFinite(expiresAt)) {
      return;
    }

    const delay = expiresAt - Date.now();
    if (delay <= 0) {
      void removeComment(comment.discord_message_id);
      return;
    }

    const timer = window.setTimeout(() => {
      expiryTimers.delete(comment.discord_message_id);
      operationQueue = operationQueue.then(() => removeComment(comment.discord_message_id));
    }, delay);
    expiryTimers.set(comment.discord_message_id, timer);
  }

  async function removeComment(id) {
    if (typeof id !== "string") {
      return;
    }

    const entry = comments.get(id);
    if (!entry) {
      return;
    }

    comments.delete(id);
    window.clearTimeout(expiryTimers.get(id));
    expiryTimers.delete(id);

    if (reducedMotion.matches || !entry.element.isConnected) {
      entry.element.remove();
    } else {
      entry.element.classList.add("comment--leaving");
      await waitForLeave(entry.element);
      entry.element.remove();
    }
    updateVisibility();
  }

  function waitForLeave(element) {
    return new Promise((resolve) => {
      let settled = false;
      const finish = () => {
        if (settled) {
          return;
        }
        settled = true;
        element.removeEventListener("transitionend", finish);
        window.clearTimeout(timeout);
        resolve();
      };
      const timeout = window.setTimeout(finish, LEAVE_FALLBACK_MS);
      element.addEventListener("transitionend", finish, { once: true });
    });
  }

  async function clearComments() {
    const ids = [...comments.keys()];
    await Promise.all(ids.map((id) => removeComment(id)));
    updateVisibility();
  }

  function showHighlight(highlight) {
    if (!isValidHighlight(highlight) || document.hidden) {
      return;
    }

    hideHighlight(true);
    const fragment = highlightTemplate.content.cloneNode(true);
    const card = fragment.querySelector(".highlight-card");
    const author = fragment.querySelector(".highlight-card__author");
    const body = fragment.querySelector(".highlight-card__body");
    const fallback = fragment.querySelector(".highlight-card__avatar-fallback");
    const avatar = fragment.querySelector(".highlight-card__avatar");
    if (!(card instanceof HTMLElement)
      || !(author instanceof HTMLElement)
      || !(body instanceof HTMLElement)
      || !(fallback instanceof HTMLElement)
      || !(avatar instanceof HTMLElement)) {
      return;
    }

    card.dataset.commentId = highlight.discord_message_id;
    author.textContent = highlight.author_name;
    body.textContent = highlight.body;
    fallback.textContent = getInitial(highlight.author_name);

    if (showAvatar) {
      const image = document.createElement("img");
      image.className = "highlight-card__avatar-image";
      image.alt = "";
      image.referrerPolicy = "no-referrer";
      image.decoding = "async";
      configureAvatar(image, highlight.avatar_url);
      avatar.append(image);
    }

    highlightElement.append(fragment);
    window.requestAnimationFrame(() => {
      if (highlightElement.hasChildNodes()) {
        highlightElement.classList.add("is-visible");
      }
    });
    highlightTimer = window.setTimeout(() => hideHighlight(false), HIGHLIGHT_DURATION_MS);
  }

  function hideHighlight(immediate) {
    window.clearTimeout(highlightTimer);
    window.clearTimeout(highlightLeaveTimer);
    highlightTimer = 0;
    highlightLeaveTimer = 0;
    highlightElement.classList.remove("is-visible");

    if (immediate || reducedMotion.matches) {
      highlightElement.replaceChildren();
      return;
    }

    highlightLeaveTimer = window.setTimeout(() => {
      highlightLeaveTimer = 0;
      highlightElement.replaceChildren();
    }, HIGHLIGHT_LEAVE_MS);
  }

  function showEffect(kind) {
    const definition = EFFECTS.get(kind);
    if (!definition || document.hidden) {
      return;
    }

    while (effectTimers.size >= MAX_EFFECT_GROUPS) {
      const oldest = effectTimers.keys().next().value;
      removeEffect(oldest);
    }

    const group = document.createElement("div");
    group.className = `effect effect--${kind}`;
    const particleCount = reducedMotion.matches ? 1 : EFFECT_PARTICLE_COUNT;
    for (let index = 0; index < particleCount; index += 1) {
      const particle = document.createElement("span");
      particle.className = definition.emoji
        ? "effect__particle effect__particle--emoji"
        : "effect__particle";
      particle.textContent = definition.label;

      if (reducedMotion.matches) {
        particle.classList.add("effect__particle--reduced");
      }
      group.append(particle);
    }

    effectsElement.append(group);
    const duration = reducedMotion.matches ? 1_200 : EFFECT_DURATION_MS;
    const timer = window.setTimeout(() => removeEffect(group), duration);
    effectTimers.set(group, timer);
  }

  function removeEffect(group) {
    if (!(group instanceof HTMLElement)) {
      return;
    }
    window.clearTimeout(effectTimers.get(group));
    effectTimers.delete(group);
    group.remove();
  }

  function clearEffects() {
    for (const [group, timer] of effectTimers) {
      window.clearTimeout(timer);
      group.remove();
    }
    effectTimers.clear();
    effectsElement.replaceChildren();
  }

  function clearTransientPresentation() {
    hideHighlight(true);
    clearEffects();
  }

  function makeTransparentImmediately() {
    overlay.classList.add("is-transparent");
    for (const timer of expiryTimers.values()) {
      window.clearTimeout(timer);
    }
    expiryTimers.clear();
    comments.clear();
    commentsElement.replaceChildren();
    clearTransientPresentation();
  }

  function updateVisibility() {
    overlay.classList.toggle("is-transparent", paused);
    commentsElement.classList.toggle("is-empty", comments.size === 0);
  }

  function applyAppearance() {
    overlay.dataset.theme = theme;
    overlay.classList.toggle("show-avatars", showAvatar);

    for (const { element, comment } of comments.values()) {
      const image = element.querySelector(".comment__avatar-image");
      if (image instanceof HTMLImageElement) {
        configureAvatar(image, comment.avatar_url);
      }
    }
  }

  function endSession() {
    terminal = true;
    paused = true;
    connectionGeneration += 1;
    snapshotReceived = false;
    window.clearTimeout(reconnectTimer);
    reconnectTimer = 0;
    stopHeartbeatWatch();
    makeTransparentImmediately();

    if (socket && socket.readyState < WebSocket.CLOSING) {
      socket.close();
    }
  }

  function isValidComment(comment) {
    return isObject(comment)
      && typeof comment.discord_message_id === "string"
      && comment.discord_message_id.length > 0
      && comment.discord_message_id.length <= 32
      && typeof comment.author_name === "string"
      && comment.author_name.length <= 256
      && typeof comment.body === "string"
      && comment.body.length <= 4_000
      && (comment.avatar_url === null || typeof comment.avatar_url === "string")
      && typeof comment.created_at === "string"
      && Number.isFinite(Date.parse(comment.created_at))
      && typeof comment.expires_at === "string"
      && Number.isFinite(Date.parse(comment.expires_at));
  }

  function isValidHighlight(highlight) {
    return isObject(highlight)
      && typeof highlight.discord_message_id === "string"
      && highlight.discord_message_id.length > 0
      && highlight.discord_message_id.length <= 32
      && typeof highlight.author_name === "string"
      && highlight.author_name.length <= 256
      && typeof highlight.body === "string"
      && highlight.body.length <= 4_000
      && (highlight.avatar_url === null || typeof highlight.avatar_url === "string");
  }

  function isExpired(expiresAt) {
    const value = Date.parse(expiresAt);
    return Number.isFinite(value) && value <= Date.now();
  }

  function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
  }
})();
