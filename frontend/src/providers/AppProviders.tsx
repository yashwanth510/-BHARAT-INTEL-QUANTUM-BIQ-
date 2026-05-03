"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { BiqDataProvider } from "./BiqDataProvider";

export function AppProviders({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000,
            refetchInterval: 60_000,
            retry: 2,
            retryDelay: (attemptIndex) =>
              Math.min(1000 * 2 ** attemptIndex, 15_000),
          },
        },
      })
  );

  return (
    <QueryClientProvider client={queryClient}>
      {/* Real-time intelligence data layer */}
      <BiqDataProvider>{children}</BiqDataProvider>
    </QueryClientProvider>
  );
}
