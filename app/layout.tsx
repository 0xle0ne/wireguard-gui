import type { Viewport } from 'next';

import '@/app/globals.css';

import { IBM_Plex_Mono } from 'next/font/google';
import { ThemeProvider } from '@/components/theme-provider';
import { Toaster } from '@/components/ui/sonner';
import { cn } from '@/lib/utils';

export const viewport: Viewport = {
  themeColor: [
    { media: '(prefers-color-scheme: light)', color: 'white' },
    { media: '(prefers-color-scheme: dark)', color: 'black' },
  ],
  width: 'device-width',
  initialScale: 1,
  viewportFit: 'cover',
};

const font = IBM_Plex_Mono({
  subsets: ['latin'],
  weight: ['400', '500', '600'],
  variable: '--font-ibm-plex-mono',
});

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head />
      <body
        className={cn(
          'bg-background font-sans overflow-x-hidden antialiased',
          font.variable,
        )}
      >
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          disableTransitionOnChange
        >
          {children}
          <Toaster />
        </ThemeProvider>
      </body>
    </html>
  );
}
