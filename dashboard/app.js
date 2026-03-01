const CONFIG = {
  baseUrl:
    window.location.hostname === "localhost"
      ? "http://localhost:3000"
      : window.location.origin,
  explorerBase: "https://explorer.solana.com/tx/",
  explorerSuffix: "?cluster=devnet",
};

const STATE = {
  agents: {}, // map of agentId -> lastKnownData
  txSignatures: new Set(),
  isPolling: false,
  eventSource: null,
  abortController: null,
};

const truncate = (s, n = 6) => {
  if (!s || s.length <= n * 2 + 3) return s || "";
  return s.slice(0, n) + "..." + s.slice(-n);
};

const lamportsToSol = (l) => {
  const s = l / 1e9;
  if (s === 0) return "0.0000";
  return s < 0.0001 ? s.toExponential(2) : s.toFixed(4);
};

const fmtToken = (amount, decimals = 9) => {
  const v = amount / Math.pow(10, decimals);
  if (v >= 1e6) return (v / 1e6).toFixed(2) + "M";
  if (v >= 1e3) return (v / 1e3).toFixed(2) + "K";
  return v.toFixed(v < 1 ? 4 : 2);
};

const fmtTime = (ts) => {
  if (!ts) return { date: "", time: "" };
  const d = new Date(typeof ts === "number" && ts < 1e12 ? ts * 1000 : ts);
  return {
    date: d.toLocaleDateString("en-GB", {
      day: "2-digit",
      month: "short",
    }),
    time: d.toLocaleTimeString("en-GB", { hour12: false }),
  };
};

const fmtClock = () =>
  new Date().toLocaleTimeString("en-GB", { hour12: false });

function createAgentElement(agent, bal, toks) {
  const init = (agent.label || "AG").slice(0, 2).toUpperCase();
  const sol = bal ? lamportsToSol(bal.sol_lamports) : null;
  const vault = bal ? lamportsToSol(bal.vault_lamports) : null;

  const div = document.createElement("div");
  div.className = "agent";
  div.id = `agent-${agent.id}`;

  let tokenHtml = "";
  if (toks && toks.length) {
    const rows = toks
      .slice(0, 3)
      .map(
        (t) =>
          `<div class="token-row"><span class="token-mint">${truncate(t.mint, 4)}</span><span class="token-amt">${fmtToken(parseFloat(t.amount), t.decimals)}</span></div>`,
      )
      .join("");
    const extra =
      toks.length > 3
        ? `<div style="font-size:0.55rem;color:var(--color-neutral);margin-top:0.15rem">+${toks.length - 3} more</div>`
        : "";
    tokenHtml = `<div class="tokens"><div class="tokens-head">Token Balances</div><div class="token-list">${rows}</div>${extra}</div>`;
  }

  div.innerHTML = `
    <div class="agent-top">
      <div class="agent-id">
        <div class="agent-avatar">${init}</div>
        <span class="agent-name">${agent.label || "Unnamed"}</span>
      </div>
      <span class="pill status-pill ${agent.is_active ? "online" : "offline"}">${agent.is_active ? "Online" : "Offline"}</span>
    </div>
    <div class="agent-key">${agent.pubkey}</div>
    <div class="agent-grid">
      <div class="metric">
        <div class="metric-label">SOL</div>
        <div class="metric-value sol-value ${sol === null ? "loading" : ""}">${sol !== null ? sol : ""}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Vault</div>
        <div class="metric-value vault-value ${vault === null ? "loading" : ""}">${vault !== null ? vault : ""}</div>
      </div>
    </div>
    ${tokenHtml}
  `;
  return div;
}

function updateAgentElement(el, agent, bal, toks) {
  // Update status pill
  const pill = el.querySelector(".status-pill");
  if (pill) {
    pill.className = `pill status-pill ${agent.is_active ? "online" : "offline"}`;
    pill.textContent = agent.is_active ? "Online" : "Offline";
  }

  // Update balances
  const sol = bal ? lamportsToSol(bal.sol_lamports) : null;
  const vault = bal ? lamportsToSol(bal.vault_lamports) : null;

  const solEl = el.querySelector(".sol-value");
  if (solEl) {
    if (sol !== null) {
      solEl.textContent = sol;
      solEl.classList.remove("loading");
    } else {
      solEl.classList.add("loading");
    }
  }

  const vaultEl = el.querySelector(".vault-value");
  if (vaultEl) {
    if (vault !== null) {
      vaultEl.textContent = vault;
      vaultEl.classList.remove("loading");
    } else {
      vaultEl.classList.add("loading");
    }
  }

  // Update tokens (simple replace for now if count changes or first 3 change)
  const tokenList = el.querySelector(".token-list");
  if (tokenList && toks) {
    const rows = toks
      .slice(0, 3)
      .map(
        (t) =>
          `<div class="token-row"><span class="token-mint">${truncate(t.mint, 4)}</span><span class="token-amt">${fmtToken(parseFloat(t.amount), t.decimals)}</span></div>`,
      )
      .join("");
    if (tokenList.innerHTML !== rows) {
      tokenList.innerHTML = rows;
    }
  }
}

function renderAgents(agents, balances, tokens) {
  const container = document.getElementById("agentGrid");
  document.getElementById("agentCount").textContent = agents.length;

  if (!agents.length) {
    container.innerHTML = `<div class="empty"><svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg><div class="empty-title">No Agents</div><div class="empty-hint">Create via API...</div></div>`;
    STATE.agents = {};
    return;
  }

  // Remove skeleton or empty state if present
  if (container.querySelector(".skel") || container.querySelector(".empty")) {
    container.innerHTML = "";
  }

  const currentIds = new Set(agents.map((a) => a.id));

  // Remove agents no longer present
  Object.keys(STATE.agents).forEach((id) => {
    if (!currentIds.has(id)) {
      const el = document.getElementById(`agent-${id}`);
      if (el) el.remove();
      delete STATE.agents[id];
    }
  });

  // Add or update agents
  agents.forEach((agent) => {
    const el = document.getElementById(`agent-${agent.id}`);
    const bal = balances[agent.id];
    const toks = tokens[agent.id];

    if (!el) {
      container.appendChild(createAgentElement(agent, bal, toks));
    } else {
      updateAgentElement(el, agent, bal, toks);
    }
    STATE.agents[agent.id] = agent;
  });
}

function renderTx(e) {
  const cls = (e.instruction_type || "").toLowerCase();
  const sig = e.tx_signature
    ? `<a href="${CONFIG.explorerBase}${e.tx_signature}${CONFIG.explorerSuffix}" target="_blank">${truncate(e.tx_signature, 8)}</a>`
    : "\u2014";
  const { date, time } = fmtTime(e.timestamp);

  const li = document.createElement("li");
  li.className = "tx";
  li.innerHTML = `
            <span class="tx-type ${cls}">${e.instruction_type || "unknown"}</span>
            <div class="tx-detail">
              <div class="tx-sig">${sig}</div>
              <div class="tx-agent">${truncate(e.agent_id, 4)}</div>
            </div>
            <span class="tx-result ${e.status}">${e.status}</span>
            <span class="tx-when"><span class="tx-when-date">${date}</span>${time}</span>
          `;
  return li;
}

function renderFeed(entries) {
  const el = document.getElementById("txFeed");
  document.getElementById("txCount").textContent = entries.length;

  if (!entries.length) {
    if (!el.querySelector(".empty")) {
      el.innerHTML = `<li class="empty"><svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/></svg><div class="empty-title">No Transactions</div><div class="empty-hint">Awaiting activity...</div></li>`;
    }
    return;
  }

  // Remove empty state
  const empty = el.querySelector(".empty");
  if (empty) empty.remove();

  // Prepend new transactions
  const newEntries = entries.filter((e) => {
    const key = e.tx_signature || `audit-${e.id}`;
    return !STATE.txSignatures.has(key);
  });
  newEntries.reverse().forEach((e) => {
    el.insertBefore(renderTx(e), el.firstChild);
    const key = e.tx_signature || `audit-${e.id}`;
    STATE.txSignatures.add(key);
  });

  // Limit feed to 50 items and prune the signature set
  while (el.children.length > 50) {
    const last = el.lastElementChild;
    if (!last) break;
    const sig = last.querySelector(".tx-sig a");
    if (sig) {
      const href = sig.getAttribute("href") || "";
      const match = href.match(/\/tx\/([^?]+)/);
      if (match) STATE.txSignatures.delete(match[1]);
    }
    el.removeChild(last);
  }
}

let API_KEY = localStorage.getItem("klave_operator_key");
const urlKey = new URLSearchParams(window.location.search).get("key");

if (urlKey) {
  API_KEY = urlKey;
  localStorage.setItem("klave_operator_key", API_KEY);
  // Clean URL to avoid leaking in history
  window.history.replaceState({}, document.title, window.location.pathname);
}

if (!API_KEY) {
  API_KEY = prompt("Enter KLAVE Operator API Key:") || "";
  if (API_KEY) {
    localStorage.setItem("klave_operator_key", API_KEY);
  }
}

async function fetchJson(path) {
  const headers = {};
  if (API_KEY) headers["x-api-key"] = API_KEY;
  const r = await fetch(CONFIG.baseUrl + path, { headers });
  if (!r.ok) throw new Error("HTTP " + r.status);
  return (await r.json()).data;
}

async function poll() {
  if (STATE.isPolling) return;
  STATE.isPolling = true;

  const dot = document.getElementById("statusDot");
  const txt = document.getElementById("statusText");

  try {
    const agents = await fetchJson("/api/v1/agents");
    const balances = {},
      tokens = {},
      entries = [];

    await Promise.all(
      agents.map(async (a) => {
        try {
          const [bal, tok, hist] = await Promise.all([
            fetchJson(`/api/v1/agents/${a.id}/balance`).catch(() => null),
            fetchJson(`/api/v1/agents/${a.id}/tokens`).catch(() => []),
            fetchJson(`/api/v1/agents/${a.id}/history`).catch(() => []),
          ]);
          balances[a.id] = bal;
          tokens[a.id] = tok;
          entries.push(...hist);
        } catch (e) {
          console.warn("agent fetch error:", e);
        }
      }),
    );

    entries.sort((a, b) => (b.timestamp || 0) - (a.timestamp || 0));

    renderAgents(agents, balances, tokens);
    renderFeed(entries);

    dot.classList.remove("offline");
    txt.textContent = `Live \u00b7 ${agents.length} agent${agents.length !== 1 ? "s" : ""} \u00b7 ${fmtClock()}`;
  } catch (err) {
    console.error("poll error:", err);
    dot.classList.add("offline");
    txt.textContent = "Offline \u2014 " + err.message;
  } finally {
    STATE.isPolling = false;
  }
}

async function connectSSE() {
  if (STATE.abortController) {
    STATE.abortController.abort();
  }

  STATE.abortController = new AbortController();
  const { signal } = STATE.abortController;

  const url = new URL(CONFIG.baseUrl + "/api/v1/events");
  const headers = { Accept: "text/event-stream" };
  if (API_KEY) headers["x-api-key"] = API_KEY;

  console.log("Connecting to SSE (via fetch)...");

  try {
    const response = await fetch(url.toString(), { headers, signal });
    if (!response.ok) {
      if (response.status === 401) {
        console.error("SSE Unauthorized. Check your API key.");
        document.getElementById("statusDot").classList.add("offline");
        document.getElementById("statusText").textContent =
          "Unauthorized (Invalid Key)";
        return;
      }
      throw new Error(`HTTP ${response.status}`);
    }

    console.log("SSE connected");
    document.getElementById("statusDot").classList.remove("offline");
    document.getElementById("statusText").textContent =
      "Live (SSE) \u00b7 " + fmtClock();

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    while (true) {
      const { value, done } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const parts = buffer.split("\n\n");
      buffer = parts.pop();

      for (const part of parts) {
        const dataLine = part.split("\n").find((l) => l.startsWith("data:"));
        if (dataLine) {
          try {
            const json = dataLine.slice(5).trim();
            if (json) {
              const { type, data } = JSON.parse(json);
              console.log("SSE Event:", type, data);
              handleServerEvent(type, data);
            }
          } catch (e) {
            console.error("SSE JSON error:", e);
          }
        }
      }
    }
  } catch (err) {
    if (err.name === "AbortError") return;
    console.warn("SSE connection error, retrying in 5s...", err);
    document.getElementById("statusDot").classList.add("offline");
    setTimeout(connectSSE, 5000);
  }
}

async function handleServerEvent(type, data) {
  switch (type) {
    case "AgentCreated":
      await poll();
      break;
    case "TransactionExecuted":
      if (data.agent_id) {
        try {
          const hist = await fetchJson(
            `/api/v1/agents/${data.agent_id}/history`,
          ).catch(() => []);
          if (hist.length) renderFeed(hist);
        } catch (e) {
          console.warn("SSE history fetch error:", e);
        }
      }
      break;
    case "BalanceUpdated":
      if (data.agent_id) {
        const agent = STATE.agents[data.agent_id];
        if (agent) {
          const el = document.getElementById(`agent-${data.agent_id}`);
          if (el) {
            const bal = {
              sol_lamports: data.sol_lamports,
              vault_lamports: data.vault_lamports,
            };
            const tok = data.tokens || [];
            updateAgentElement(el, agent, bal, tok);
          }
        }
        try {
          const hist = await fetchJson(
            `/api/v1/agents/${data.agent_id}/history`,
          ).catch(() => []);
          if (hist.length) renderFeed(hist);
        } catch (_) { }
      }
      break;
    case "Message":
      console.log("Server message:", data.text);
      break;
  }
}

// Initial load
poll().then(() => {
  connectSSE();
});

// Update health link
document.getElementById("healthLink").href = CONFIG.baseUrl + "/health";

// Theme Toggle
const themeToggle = document.getElementById("themeToggle");
const body = document.body;

const savedTheme = localStorage.getItem("theme");
if (savedTheme === "light") {
  body.classList.add("light-mode");
}

themeToggle.addEventListener("click", () => {
  const isLight = body.classList.toggle("light-mode");
  localStorage.setItem("theme", isLight ? "light" : "dark");
});
