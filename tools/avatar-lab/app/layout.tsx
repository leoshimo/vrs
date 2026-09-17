import type { ReactNode } from 'react';
import './globals.css';
export const metadata = {
  title: 'Avatar Lab',
  description: 'Avatar expressions and shader recipe',
};
export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
