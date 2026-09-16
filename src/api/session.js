// Holds the current session token in memory. Commands that need a session read
// it from here; the real Tauri commands take it as their first argument.

let token = null;

export function getToken() {
  return token;
}

export function setToken(value) {
  token = value;
}

export function clearToken() {
  token = null;
}
