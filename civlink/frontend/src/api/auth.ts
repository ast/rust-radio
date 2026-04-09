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
  return data.token;
}
