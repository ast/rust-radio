const TOKEN_KEY = "civlink_token";

interface LoginResponse {
  token: string;
}

export async function login(
  username: string,
  password: string,
): Promise<string> {
  console.log(`[auth] logging in as '${username}'`);

  const response = await fetch("/api/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });

  if (!response.ok) {
    console.error(`[auth] login failed: ${response.status} ${response.statusText}`);
    throw new Error("Login failed");
  }

  const data: LoginResponse = await response.json();
  console.log("[auth] login successful, token received");
  sessionStorage.setItem(TOKEN_KEY, data.token);
  return data.token;
}

export async function restoreSession(): Promise<string | null> {
  const token = sessionStorage.getItem(TOKEN_KEY);
  if (!token) return null;

  try {
    const response = await fetch("/api/validate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ token }),
    });
    if (response.ok) {
      console.log("[auth] session restored from storage");
      return token;
    }
  } catch {
    // Server unreachable — discard stale token
  }

  sessionStorage.removeItem(TOKEN_KEY);
  return null;
}

export function clearSession(): void {
  sessionStorage.removeItem(TOKEN_KEY);
}
