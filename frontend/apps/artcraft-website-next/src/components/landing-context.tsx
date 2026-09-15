"use client";

import { useEffect } from "react";
import { logWebReferral } from "@/lib/api";
import { captureLandingContext, getReferrer } from "@/lib/referral";

const REFERRAL_LOGGED_KEY = "referral_logged";

// Site-wide attribution bootstrap (ported from the Vite site's main.tsx):
// persist the landing context on every page load, and log the referral to
// the API once per browser session.
export default function LandingContext() {
  useEffect(() => {
    captureLandingContext();
    try {
      if (sessionStorage.getItem(REFERRAL_LOGGED_KEY)) return;
      sessionStorage.setItem(REFERRAL_LOGGED_KEY, "1");
    } catch {
      return;
    }
    logWebReferral(getReferrer()).catch(() => {});
  }, []);

  return null;
}
