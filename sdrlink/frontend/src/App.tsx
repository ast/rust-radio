import { Component, createSignal, onMount, Show } from "solid-js";
import Login from "./components/Login";
import SpectrumView from "./components/SpectrumView";

const TOKEN_KEY = "sdrlink.token";

const App: Component = () => {
  const [token, setToken] = createSignal<string | null>(null);
  const [ready, setReady] = createSignal(false);

  onMount(async () => {
    const saved = localStorage.getItem(TOKEN_KEY);
    if (saved && (await validate(saved))) {
      setToken(saved);
    } else if (saved) {
      localStorage.removeItem(TOKEN_KEY);
    }
    setReady(true);
  });

  const onLogin = (t: string) => {
    localStorage.setItem(TOKEN_KEY, t);
    setToken(t);
  };

  const onLogout = () => {
    localStorage.removeItem(TOKEN_KEY);
    setToken(null);
  };

  return (
    <Show when={ready()}>
      <Show when={token()} fallback={<Login onLogin={onLogin} />}>
        <SpectrumView token={token()!} onLogout={onLogout} />
      </Show>
    </Show>
  );
};

async function validate(token: string): Promise<boolean> {
  try {
    const res = await fetch("/api/validate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ token }),
    });
    return res.ok;
  } catch {
    return false;
  }
}

export default App;
