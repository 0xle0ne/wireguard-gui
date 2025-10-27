'use client';

import { useCallback, useMemo, useState } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import * as z from 'zod';

import { ProfilePartial } from '@/types/profile';
import { createProfile, updateProfile } from '@/lib/effects';
import { Button } from '@/components/ui/button';
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';

import { Textarea } from './ui/textarea';

const formSchema = z.object({
  name: z.string().min(2, {
    message: 'Name must be at least 2 characters.',
  }),
  content: z.string().min(8, {
    message: 'Content must be at least 8 characters.',
  }),
});

const contentPlaceholder = `[Interface]
Address =
PrivateKey =
ListenPort = 51820
DNS =

[Peer]
PublicKey =
PresharedKey =
Endpoint =
AllowedIPs =
`;

interface ProfileFormProps {
  data?: ProfilePartial | null;
  editId?: string | null;
  afterSubmit?: (profile: ProfilePartial) => void;
}

export default function ProfileForm({
  data,
  editId,
  afterSubmit,
}: ProfileFormProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const defaultValues = useMemo(
    () =>
      data || {
        name: '',
        content: '',
      },
    [data],
  );

  console.log({ defaultValues });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: defaultValues,
  });

  const onSubmit = useCallback(
    (profile: z.infer<typeof formSchema>) => {
      setIsLoading(true);
      if (editId) {
        updateProfile(
          editId,
          profile,
          () => afterSubmit?.(profile),
          (err) => setError(err.message ? err.message : err),
          () => setIsLoading(false),
        );
      } else {
        createProfile(
          profile,
          () => afterSubmit?.(profile),
          (err) => setError(err.message ? err.message : err),
          () => setIsLoading(false),
        );
      }
    },
    [editId, afterSubmit],
  );

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem>
              <FormLabel>Name</FormLabel>
              <FormControl>
                <Input placeholder="wgnet0" {...field} disabled={!!editId} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="content"
          render={({ field }) => (
            <FormItem className="mb-4">
              <FormLabel>Content</FormLabel>
              <FormControl>
                <Textarea
                  className="h-[280px] resize-none"
                  placeholder={contentPlaceholder}
                  {...field}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        {error ? <div className="text-sm text-red-500">{error}</div> : null}
        <Button
          variant="outline"
          className="w-full cursor-pointer"
          type="submit"
          disabled={isLoading}
        >
          {isLoading ? 'Loading...' : 'Save'}
        </Button>
      </form>
    </Form>
  );
}
