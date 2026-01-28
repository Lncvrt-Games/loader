'use client'

import { Lexend } from 'next/font/google'
import './globals.css'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect } from 'react'

const lexend = Lexend({
  subsets: ['latin']
})

export default function RootLayout ({
  children
}: Readonly<{
  children: React.ReactNode
}>) {
  const handleMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    if ((e.target as HTMLElement).closest('button')) return
    getCurrentWindow().startDragging()
  }

  useEffect(() => {
    const handler = (e: MouseEvent) => e.preventDefault()
    document.addEventListener('contextmenu', handler)
    return () => document.removeEventListener('contextmenu', handler)
  }, [])

  useEffect(() => {
    document.body.addEventListener('mousedown', handleMouseDown as any)
    return () => document.body.removeEventListener('mousedown', handleMouseDown as any)
  }, [])

  return (
    <html lang='en'>
      <body className={lexend.className}>
        <div className='w-max h-screen'>{children}</div>
      </body>
    </html>
  )
}
