import { useEffect, useState } from "react";
import { controllerKeys, type KeyInfo } from "../../lib/controllers";

/** The keys and mouse buttons a button can be converted to (empty until the backend answers). */
export function useControllerKeys(): KeyInfo[] {
  const [keys, setKeys] = useState<KeyInfo[]>([]);
  useEffect(() => {
    let current = true;
    void controllerKeys().then((list) => current && setKeys(list));
    return () => {
      current = false;
    };
  }, []);
  return keys;
}
