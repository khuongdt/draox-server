import { useState, useCallback } from 'react';

/** Global auth model — stores current user and provides login/logout helpers. */
export default function useAuthModel() {
  const [currentUser, setCurrentUser] = useState<API.CurrentUser | null>(null);

  /** Persist token to localStorage and set the current user in state. */
  const login = useCallback((user: API.CurrentUser) => {
    localStorage.setItem('draox_token', user.token);
    localStorage.setItem('draox_role', user.role);
    setCurrentUser(user);
  }, []);

  /** Clear auth data from localStorage and reset state. */
  const logout = useCallback(() => {
    localStorage.removeItem('draox_token');
    localStorage.removeItem('draox_role');
    setCurrentUser(null);
  }, []);

  return { currentUser, setCurrentUser, login, logout };
}
