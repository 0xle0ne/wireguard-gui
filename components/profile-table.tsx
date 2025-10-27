'use client';

import React from 'react';
import { usePathname, useRouter, useSearchParams } from 'next/navigation';
import { ColumnDef } from '@tanstack/react-table';
import { DeleteIcon, Edit, Rocket } from 'lucide-react';

import type { Profile } from '@/types/profile';
import { useDebounce, useProfiles } from '@/lib/effects';

import { ProfileDialogDelete } from './profile-dialog-delete';
import { ProfileDialogForm } from './profile-dialog-form';
import { DataTable } from './ui/data-table';
import { Input } from './ui/input';

export interface ProfileTableProps {
  current?: string | null;
  onConnect: (profile: string) => () => void;
}

function ProfileNameHeader() {
  return <p className="font-bold">Profiles</p>;
}

function ProfileNameCell({
  row,
  current,
}: {
  row: any;
  current?: string | null;
}) {
  return (
    <div className="flex items-center h-6">
      {current === row.original.name ? (
        <div className="size-4 animate-pulse rounded-full bg-green-500" />
      ) : (
        <div className="size-4 rounded-full bg-red-500" />
      )}
      <p className="ml-2">{row.original.name}</p>
    </div>
  );
}

function ProfileActions({
  row,
  onConnect,
}: {
  row: any;
  onConnect: (profile: string) => () => void;
}) {
  const profile = row.original;
  const router = useRouter();
  const pathname = usePathname();

  const onDelete = React.useCallback(() => {
    const query = new URLSearchParams();
    query.set('d', profile.name);
    router.push(`${pathname}?${query.toString()}`);
  }, [pathname, profile.name, router]);

  const onEdit = React.useCallback(() => {
    const query = new URLSearchParams();
    query.set('e', profile.name);
    router.push(`${pathname}?${query.toString()}`);
  }, [pathname, profile.name, router]);

  return (
    <div className="flex flex-row">
      <button
        className="flex w-full justify-between cursor-pointer"
        title="Delete"
        onClick={onDelete}
      >
        <DeleteIcon className="ml-2 size-4 text-red-500" />
      </button>
      <button
        className="flex w-full justify-between cursor-pointer"
        title="Edit"
        onClick={onEdit}
      >
        <Edit className="ml-2 size-4 text-blue-500" />
      </button>
      <button
        className="flex w-full justify-between cursor-pointer"
        title="Connect"
        onClick={onConnect(profile.name)}
      >
        <Rocket className="ml-2 size-4 text-green-500" />
      </button>
    </div>
  );
}

export function ProfileTable({ current, onConnect }: ProfileTableProps) {
  const router = useRouter();
  const pathname = usePathname();
  const qs = useSearchParams();
  const editId = qs.get('e');
  const [data, , , setData, fetchData] = useProfiles();
  const [filter, setFilter] = React.useState('');
  const [debounceFilter, setDebounceFilter] = useDebounce(500, filter);

  const columns = React.useMemo<ColumnDef<Profile>[]>(
    () => [
      {
        accessorKey: 'name',
        header: ProfileNameHeader,
        cell: ({ row }) => <ProfileNameCell row={row} current={current} />,
      },
      {
        id: 'actions',
        cell: ({ row }) => <ProfileActions row={row} onConnect={onConnect} />,
      },
    ],
    [current, onConnect],
  );

  const tableData = React.useMemo(() => {
    // sort by name
    let sortedData = data;
    if (debounceFilter) {
      sortedData = (data || []).filter((p) =>
        p.name.toLowerCase().includes(debounceFilter.toLowerCase()),
      );
    } else {
      sortedData = (data || []).sort((a: any, b: any) =>
        a.name.localeCompare(b.name),
      );
    }
    // move current profile to the top
    return sortedData.sort((a: any) => (a.name === current ? -1 : 1));
  }, [current, data, debounceFilter]);

  const editProfile = React.useMemo(
    () => tableData.find(({ name }) => name === editId) || null,
    [editId, tableData],
  );

  const onDelete = React.useCallback(() => fetchData(), [fetchData]);

  const onProfileFormOpenChange = React.useCallback(
    (o: boolean) => {
      if (o) return;
      router.replace(pathname);
    },
    [router, pathname],
  );

  const afterProfileForm = React.useCallback(
    (profile: Profile) => {
      if (editId) {
        setData((d) => d?.map((p) => (p.name === editId ? profile : p)));
      } else {
        setData((d) => [...(d || []), profile]);
      }
      router.replace(pathname);
    },
    [editId, router, pathname, setData],
  );

  const onSearchChange = React.useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      const value = event.currentTarget.value;
      setDebounceFilter(value || '');
      setFilter(value || '');
    },
    [setDebounceFilter],
  );

  return (
    <div>
      <ProfileDialogDelete onDelete={onDelete} />
      <div className="relative mr-2 flex justify-end">
        <ProfileDialogForm
          data={editProfile}
          editId={editId}
          afterSubmit={afterProfileForm}
          onOpenChange={onProfileFormOpenChange}
          className="absolute top-14 z-10 cursor-pointer"
        />
      </div>
      <Input
        placeholder="Search"
        className="mb-2"
        value={filter}
        onChange={onSearchChange}
      />
      <DataTable columns={columns} data={tableData} />
    </div>
  );
}
