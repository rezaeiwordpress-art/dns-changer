import { useCallback, useState } from 'react';
import type { ErrorCode, ToastNotification } from '../types/dns';

export function useNotifications() {
  const [notifications, setNotifications] = useState<ToastNotification[]>([]);

  const dismissNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((item) => item.id !== id));
  }, []);

  const pushNotification = useCallback(
    (
      variant: ToastNotification['variant'],
      title: string,
      message: string,
      errorCode?: ErrorCode,
    ) => {
      const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      setNotifications((prev) => [
        { id, variant, title, message, errorCode },
        ...prev.slice(0, 3),
      ]);

      window.setTimeout(() => {
        setNotifications((prev) => prev.filter((item) => item.id !== id));
      }, 6000);
    },
    [],
  );

  return {
    notifications,
    pushNotification,
    dismissNotification,
  };
}
