interface LoginResponse {
  token: string;
}

export async function login(
  username: string,
  password: string,
): Promise<string> {
  const response = await fetch("/api/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });

  if (!response.ok) {
    throw new Error("Login failed");
  }

  const data: LoginResponse = await response.json();
  return data.token;
}
