import { useCallback, useEffect, useState } from 'react';
import { usePathname, useRouter, useSearchParams } from 'next/navigation';

import { deleteProfile } from '@/lib/effects';

import { AlertConfirm } from './ui/alert-confirm';

export function ProfileDialogDelete({ onDelete }: { onDelete: () => void }) {
  const router = useRouter();
  const pathname = usePathname();
  const qs = useSearchParams();
  const [isOpen, setIsOpen] = useState(false);
  const deleteId = qs.get('d');

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    if (deleteId && !isOpen) return setIsOpen(true);
    if (!deleteId && isOpen) return setIsOpen(false);
  }, [isOpen, deleteId]);

  const setOpen = useCallback(() => {
    router.replace(pathname);
  }, [pathname, router]);

  const onConfirm = useCallback(() => {
    deleteProfile(deleteId as string, undefined, undefined, () => {
      onDelete();
      setOpen();
    });
  }, [deleteId, setOpen, onDelete]);

  return (
    <AlertConfirm
      isOpen={isOpen}
      setOpen={setOpen}
      onConfirm={onConfirm}
      title="Are you absolutely sure?"
      description="This action cannot be undone. This will permanently delete your profile and remove it's data"
    />
  );
}
