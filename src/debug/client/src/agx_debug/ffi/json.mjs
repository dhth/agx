export function stringify(value) {
  return JSON.stringify(value, null, 2);
}

export function tryPrettyPrint(str) {
  try {
    const parsed = JSON.parse(str);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return str;
  }
}
