"use client";

import { useEffect, useState } from "react";
import {
  getSession,
  listActiveSubscriptions,
  type SessionUser,
} from "./api";

// Session + ArtCraft subscription, fetched once per page load and shared by
// every consumer (the plan grid and the credits cell both need it). Server
// HTML renders the logged-out state; hydration upgrades it.
export type Account = {
  /** null until the request resolves. */
  user: SessionUser | null;
  /** Active ArtCraft plan slug, or null when none / logged out. */
  activePlanSlug: string | null;
  loading: boolean;
};

const LOGGED_OUT: Account = { user: null, activePlanSlug: null, loading: false };

let pending: Promise<Account> | null = null;

function loadAccount(): Promise<Account> {
  pending ??= (async () => {
    const session = await getSession();
    if (!session.success || !session.data.loggedIn || !session.data.user) {
      return LOGGED_OUT;
    }
    const subs = await listActiveSubscriptions();
    const artcraft = subs.success
      ? subs.data.find((s) => s.namespace === "artcraft")
      : undefined;
    return {
      user: session.data.user,
      activePlanSlug: artcraft?.product_slug ?? null,
      loading: false,
    };
  })();
  return pending;
}

export function useAccount(): Account {
  const [account, setAccount] = useState<Account>({
    user: null,
    activePlanSlug: null,
    loading: true,
  });

  useEffect(() => {
    let live = true;
    loadAccount().then((a) => {
      if (live) setAccount(a);
    });
    return () => {
      live = false;
    };
  }, []);

  return account;
}
