import { Globe2Icon } from 'lucide-react';

export interface AppLoaderProps {
  isOpen?: boolean;
  kind?: 'Connecting' | 'Disconnecting';
  message?: string;
}

const hiddenStyle = { display: 'none', zIndex: -1, width: 0, height: 0 };

export function AppLoader({ isOpen, message, kind }: AppLoaderProps) {
  return (
    <div
      className="bg-background fixed z-50 flex size-full items-center justify-center backdrop-blur"
      style={isOpen ? {} : hiddenStyle}
    >
      <div className="jusify-center flex flex-col items-center">
        {kind === 'Connecting' ? (
          <Globe2Icon className="size-10 animate-pulse text-green-500" />
        ) : (
          <Globe2Icon className="size-10 animate-pulse text-red-500" />
        )}
        <p className="mt-2 font-bold">{message}</p>
      </div>
    </div>
  );
}
