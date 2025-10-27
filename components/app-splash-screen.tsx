import Image from 'next/image';

export function AppSplashScreen() {
  return (
    <div className="bg-background w-full h-full backdrop-blur-lg fixed z-50 flex size-full items-center justify-center">
      <Image
        className="animate-pulse select-none"
        alt="wireguard"
        src="/img/wireguard.png"
        width={200}
        height={200}
      />
    </div>
  );
}
