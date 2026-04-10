import { type Component, Show, createSignal, onMount } from "solid-js";
import Login from "./components/Login";
import RadioPanel from "./components/RadioPanel";
import { restoreSession } from "./api/auth";

const App: Component = () => {
  const [token, setToken] = createSignal<string | null>(null);
  const [ready, setReady] = createSignal(false);

  console.log("[app] civlink frontend loaded");

  onMount(async () => {
    const saved = await restoreSession();
    if (saved) setToken(saved);
    setReady(true);
  });

  return (
    <div class="app">
      <Show when={ready()}>
        <Show when={token()} fallback={<Login onLogin={setToken} />}>
          <RadioPanel token={token()!} />
        </Show>
      </Show>
    </div>
  );
};

export default App;
