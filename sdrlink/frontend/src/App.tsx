import { Component, createSignal, Show } from "solid-js";
import Login from "./components/Login";
import SpectrumView from "./components/SpectrumView";

const TOKEN_KEY = "sdrlink.token";

const App: Component = () => {
  const [token, setToken] = createSignal<string | null>(
    localStorage.getItem(TOKEN_KEY),
  );

  const onLogin = (t: string) => {
    localStorage.setItem(TOKEN_KEY, t);
    setToken(t);
  };

  const onLogout = () => {
    localStorage.removeItem(TOKEN_KEY);
    setToken(null);
  };

  return (
    <Show when={token()} fallback={<Login onLogin={onLogin} />}>
      <SpectrumView token={token()!} onLogout={onLogout} />
    </Show>
  );
};

export default App;
