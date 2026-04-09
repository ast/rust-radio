import { type Component, createSignal } from "solid-js";
import { login } from "../api/auth";

interface LoginProps {
  onLogin: (token: string) => void;
}

const Login: Component<LoginProps> = (props) => {
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError(null);
    try {
      const token = await login(username(), password());
      props.onLogin(token);
    } catch {
      setError("Invalid credentials");
    }
  };

  return (
    <form class="login-form" onSubmit={handleSubmit}>
      <h1>civlink</h1>
      {error() && <p class="error">{error()}</p>}
      <input
        type="text"
        placeholder="Username"
        value={username()}
        onInput={(e) => setUsername(e.currentTarget.value)}
      />
      <input
        type="password"
        placeholder="Password"
        value={password()}
        onInput={(e) => setPassword(e.currentTarget.value)}
      />
      <button type="submit">Login</button>
    </form>
  );
};

export default Login;
