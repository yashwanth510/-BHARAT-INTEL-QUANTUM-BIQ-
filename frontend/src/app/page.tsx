'use client';

import dynamic from 'next/dynamic';

const BiqMap = dynamic(() => import('@/components/BiqMap'), {
  ssr: false,
  loading: () => <div className="loading-msg">Loading BIQ…</div>,
});

export default function Home() {
  return <BiqMap />;
}
