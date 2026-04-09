import { type Component, Show, createSignal } from "solid-js";
import Login from "./components/Login";
import RadioPanel from "./components/RadioPanel";

const App: Component = () => {
  const [token, setToken] = createSignal<string | null>(null);

  return (
    <div class="app">
      <Show when={token()} fallback={<Login onLogin={setToken} />}>
        <RadioPanel token={token()!} />
      </Show>
    </div>
  );
};

export default App;
