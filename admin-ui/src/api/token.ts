const tokenKey = "starary_server_admin_token";

export function getStoredToken() {
  return localStorage.getItem(tokenKey) ?? sessionStorage.getItem(tokenKey);
}

export function storeToken(token: string) {
  localStorage.setItem(tokenKey, token);
}

export function storeTokenForSession(token: string) {
  sessionStorage.setItem(tokenKey, token);
}

export function clearStoredToken() {
  localStorage.removeItem(tokenKey);
  sessionStorage.removeItem(tokenKey);
}
