'use client'

import Image from 'next/image'
import Icon from './assets/Icon.png'
import { useEffect, useState } from 'react'
import axios from 'axios'
import { app } from '@tauri-apps/api'
import { invoke } from '@tauri-apps/api/core'
import { LauncherUpdate } from './types/LauncherUpdate'
import { openUrl } from '@tauri-apps/plugin-opener'
import { arch, platform } from '@tauri-apps/plugin-os'

export default function Home () {
  const [state, setState] = useState<string>('Loading...')

  useEffect(() => {
    ;(async () => {
      setState('Checking for updates...')

      let updaterLatestRequest
      let launcherLatestRequest
      let launcherUpdateData: LauncherUpdate | null

      try {
        updaterLatestRequest = await axios.get(
          'https://games.lncvrt.xyz/api/launcher/loader/latest'
        )
        launcherLatestRequest = await axios.get(
          'https://games.lncvrt.xyz/api/launcher/latest'
        )
      } catch {
        setState('Failed. Check internet connection.')
        return
      }

      if (
        updaterLatestRequest.status !== 200 ||
        launcherLatestRequest.status !== 200
      ) {
        setState('Failed. Try again later.')
        return
      }

      const version = await app.getVersion()
      if (version !== updaterLatestRequest.data) {
        setState('Loader update required')
        return
      }

      const isLatest = await invoke('check_latest_ver', {
        version: launcherLatestRequest.data
      })
      if (isLatest == '1') {
        setState('Starting...')
      } else {
        setState('Downloading new update...')
        try {
          const launcherUpdateRequest = await axios.get(
            `https://games.lncvrt.xyz/api/launcher/loader/update-data?platform=${platform()}&arch=${arch()}`
          )
          launcherUpdateData = launcherUpdateRequest.data
        } catch {
          setState('Failed. Check internet connection.')
          return
        }
        if (!launcherUpdateData) return
        const downloadResult = await invoke('download', {
          url: launcherUpdateData.downloadUrl,
          name: launcherLatestRequest.data,
          hash: launcherUpdateData.sha512sum
        })
        if (downloadResult == '-1') {
          setState('Failed. Check internet connection.')
          return
        } else if (downloadResult == '-2') {
          setState('File integrity check failed.')
          return
        }
        setState('Starting...')
      }

      invoke('load', {
        name: launcherLatestRequest.data
      })
    })()
  }, [])

  return (
    <>
      <div className='absolute left-1/2 top-[20%] -translate-x-1/2 flex flex-col items-center'>
        <Image src={Icon} width={128} height={128} alt='' draggable={false} />
        <div
          className={`${
            state !== 'Loader update required' ? 'mt-10' : 'mt-4'
          } text-center`}
        >
          <p className='whitespace-nowrap'>{state}</p>
          <button
            hidden={state !== 'Loader update required'}
            className='mt-4'
            onClick={async () =>
              await openUrl('https://games.lncvrt.xyz/download')
            }
          >
            Update
          </button>
        </div>
      </div>
    </>
  )
}
