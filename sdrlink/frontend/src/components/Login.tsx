import { Component, createSignal } from "solid-js";

interface Props {
  onLogin: (token: string) => void;
}

const Login: Component<Props> = (props) => {
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const submit = async (e: Event) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const res = await fetch("/api/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username: username(), password: password() }),
      });
      if (!res.ok) {
        setError("invalid credentials");
        return;
      }
      const body = (await res.json()) as { token: string };
      props.onLogin(body.token);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form onSubmit={submit}>
      <h1>sdrlink</h1>
      <div>
        <input
          placeholder="username"
          value={username()}
          onInput={(e) => setUsername(e.currentTarget.value)}
          autocomplete="username"
        />
      </div>
      <div>
        <input
          type="password"
          placeholder="password"
          value={password()}
          onInput={(e) => setPassword(e.currentTarget.value)}
          autocomplete="current-password"
        />
      </div>
      <button type="submit" disabled={busy()}>
        {busy() ? "…" : "login"}
      </button>
      {error() && <div style={{ color: "tomato" }}>{error()}</div>}
    </form>
  );
};

export default Login;
