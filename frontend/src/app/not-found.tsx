import Link from 'next/link'

export default function NotFound() {
  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-[#02040a] text-white p-4 text-center">
      <h2 className="text-4xl font-bold text-threat-red mb-4 tracking-tighter uppercase">404 - Sector Not Found</h2>
      <p className="text-white/60 mb-8 max-w-md font-mono text-sm">
        The intelligence module you are looking for is either classified or does not exist in this sector.
      </p>
      <Link 
        href="/"
        className="px-6 py-2 bg-intel-blue/20 border border-intel-blue/50 text-intel-blue hover:bg-intel-blue/40 transition-colors font-bold uppercase text-xs tracking-widest"
      >
        Return to Command Center
      </Link>
    </div>
  )
}
