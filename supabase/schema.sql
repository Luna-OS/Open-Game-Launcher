-- ======================================================================
-- MAZ Launcher – Datenbank für Konten, Freunde, Chat und "spielt gerade"
--
-- Einmal im Supabase-Dashboard ausführen:
--   SQL Editor → New query → diesen ganzen Text einfügen → "Run"
-- Kann gefahrlos erneut ausgeführt werden (z. B. nach einem Update).
--
-- Sicherheit: Alle Tabellen sind gesperrt (Row Level Security ohne
-- direkte Rechte). Der Launcher arbeitet nur über die Funktionen unten,
-- die immer prüfen, wer angemeldet ist. So kann niemand fremde Chats
-- lesen oder sehen, was Nicht-Freunde gerade spielen.
-- ======================================================================

-- ---------- Profile ----------
create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  username text unique not null,
  display_name text,
  avatar_url text,
  playing text,
  playing_since timestamptz,
  last_seen timestamptz default now(),
  created_at timestamptz default now()
);
create unique index if not exists profiles_username_lower on public.profiles (lower(username));

-- ---------- Freundschaften ----------
create table if not exists public.friendships (
  id bigint generated always as identity primary key,
  requester uuid not null references public.profiles(id) on delete cascade,
  addressee uuid not null references public.profiles(id) on delete cascade,
  status text not null default 'pending' check (status in ('pending', 'accepted')),
  created_at timestamptz default now(),
  check (requester <> addressee)
);
create unique index if not exists friendships_pair
  on public.friendships (least(requester, addressee), greatest(requester, addressee));

-- ---------- Nachrichten ----------
create table if not exists public.messages (
  id bigint generated always as identity primary key,
  sender uuid not null references public.profiles(id) on delete cascade,
  receiver uuid not null references public.profiles(id) on delete cascade,
  body text not null check (length(body) between 1 and 2000),
  created_at timestamptz default now(),
  read_at timestamptz
);
create index if not exists messages_receiver_unread on public.messages (receiver, read_at);
create index if not exists messages_pair on public.messages (least(sender, receiver), greatest(sender, receiver), id);

-- ---------- Alles sperren: Zugriff nur über die Funktionen ----------
alter table public.profiles enable row level security;
alter table public.friendships enable row level security;
alter table public.messages enable row level security;
revoke all on public.profiles, public.friendships, public.messages from anon, authenticated;

-- ---------- Profil automatisch anlegen (Google, GitHub oder E-Mail) ----------
create or replace function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public as $$
declare
  base text;
  candidate text;
  n int := 0;
begin
  base := coalesce(
    new.raw_user_meta_data->>'username',
    new.raw_user_meta_data->>'user_name',
    new.raw_user_meta_data->>'preferred_username',
    split_part(new.email, '@', 1),
    'spieler');
  base := left(regexp_replace(base, '[^A-Za-z0-9_.-]', '', 'g'), 20);
  if length(base) < 3 then base := base || 'spieler'; end if;
  candidate := base;
  while exists (select 1 from profiles where lower(username) = lower(candidate)) loop
    n := n + 1;
    candidate := base || n::text;
  end loop;
  insert into profiles (id, username, display_name, avatar_url)
  values (
    new.id,
    candidate,
    left(coalesce(nullif(trim(coalesce(new.raw_user_meta_data->>'full_name', new.raw_user_meta_data->>'name')), ''), candidate), 40),
    -- Nur echte Profilbilder von Google/GitHub übernehmen (keine fremden Tracking-Links)
    case when new.raw_user_meta_data->>'avatar_url' ~ '^https://(lh[0-9]\.googleusercontent\.com|avatars\.githubusercontent\.com)/[^"<>\s]+$' and length(new.raw_user_meta_data->>'avatar_url') <= 500
         then new.raw_user_meta_data->>'avatar_url' end)
  on conflict (id) do nothing;
  return new;
end $$;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created after insert on auth.users
  for each row execute function public.handle_new_user();

-- ---------- Funktionen für den Launcher ----------

-- Eigenes Profil
create or replace function public.my_profile()
returns table (id uuid, username text, display_name text, avatar_url text)
language sql stable security definer set search_path = public as $$
  select p.id, p.username, p.display_name, p.avatar_url from profiles p where p.id = auth.uid();
$$;

-- Profil ändern: 'ok' | 'taken' | 'invalid'
create or replace function public.update_profile(p_username text, p_display_name text)
returns text language plpgsql security definer set search_path = public as $$
begin
  if auth.uid() is null then return 'invalid'; end if;
  if p_username !~ '^[A-Za-z0-9_.-]{3,24}$' then return 'invalid'; end if;
  if exists (select 1 from profiles where lower(username) = lower(p_username) and id <> auth.uid()) then
    return 'taken';
  end if;
  update profiles
     set username = p_username,
         display_name = nullif(left(trim(coalesce(p_display_name, '')), 40), '')
   where id = auth.uid();
  return 'ok';
end $$;

-- "Ich bin online" + was ich gerade spiele (alle ~30 Sekunden)
create or replace function public.heartbeat(p_playing text)
returns void language plpgsql security definer set search_path = public as $$
declare
  g text := nullif(left(trim(coalesce(p_playing, '')), 120), '');
begin
  update profiles
     set last_seen = now(),
         playing_since = case when g is distinct from playing then (case when g is null then null else now() end) else playing_since end,
         playing = g
   where id = auth.uid();
end $$;

-- Offline melden (beim Schließen / Abmelden)
create or replace function public.go_offline()
returns void language sql security definer set search_path = public as $$
  update profiles set last_seen = now() - interval '1 hour', playing = null, playing_since = null where id = auth.uid();
$$;

-- Nutzer suchen (nur Benutzername / Anzeigename)
create or replace function public.search_users(p_query text)
returns table (id uuid, username text, display_name text, avatar_url text)
language sql stable security definer set search_path = public as $$
  select p.id, p.username, p.display_name, p.avatar_url
    from profiles p
   where p.id <> auth.uid()
     and length(trim(coalesce(p_query, ''))) >= 2
     and (p.username ilike '%' || trim(p_query) || '%' or p.display_name ilike '%' || trim(p_query) || '%')
   order by (lower(p.username) = lower(trim(p_query))) desc, p.username
   limit 20;
$$;

-- Freundesliste inkl. Anfragen, Online-Status, "spielt gerade" und ungelesenen Nachrichten
create or replace function public.friends_overview()
returns table (
  friendship_id bigint, status text, incoming boolean,
  id uuid, username text, display_name text, avatar_url text,
  playing text, playing_since timestamptz, online boolean, unread int)
language sql stable security definer set search_path = public as $$
  select f.id, f.status, (f.addressee = auth.uid()),
         p.id, p.username, p.display_name, p.avatar_url,
         case when f.status = 'accepted' and p.last_seen > now() - interval '2 minutes' then p.playing end,
         case when f.status = 'accepted' and p.last_seen > now() - interval '2 minutes' then p.playing_since end,
         (f.status = 'accepted' and p.last_seen > now() - interval '2 minutes'),
         (select count(*)::int from messages m where m.sender = p.id and m.receiver = auth.uid() and m.read_at is null)
    from friendships f
    join profiles p on p.id = case when f.requester = auth.uid() then f.addressee else f.requester end
   where auth.uid() in (f.requester, f.addressee)
   order by 10 desc, p.username;
$$;

-- Freundschaftsanfrage: 'sent' | 'accepted' | 'exists' | 'self' | 'not_found'
create or replace function public.send_friend_request(p_user uuid)
returns text language plpgsql security definer set search_path = public as $$
declare
  existing friendships%rowtype;
begin
  if auth.uid() is null or p_user is null then return 'not_found'; end if;
  if p_user = auth.uid() then return 'self'; end if;
  if not exists (select 1 from profiles where id = p_user) then return 'not_found'; end if;
  select * into existing from friendships
   where least(requester, addressee) = least(auth.uid(), p_user)
     and greatest(requester, addressee) = greatest(auth.uid(), p_user);
  if found then
    if existing.status = 'pending' and existing.addressee = auth.uid() then
      update friendships set status = 'accepted' where id = existing.id;
      return 'accepted';
    end if;
    return 'exists';
  end if;
  -- Schutz vor Spam: höchstens 30 neue Anfragen pro Stunde
  if (select count(*) from friendships where requester = auth.uid() and created_at > now() - interval '1 hour') >= 30 then
    raise exception 'rate limit';
  end if;
  insert into friendships (requester, addressee) values (auth.uid(), p_user);
  return 'sent';
end $$;

-- Anfrage annehmen oder ablehnen
create or replace function public.respond_friend_request(p_id bigint, p_accept boolean)
returns void language plpgsql security definer set search_path = public as $$
begin
  if p_accept then
    update friendships set status = 'accepted' where id = p_id and addressee = auth.uid() and status = 'pending';
  else
    delete from friendships where id = p_id and addressee = auth.uid() and status = 'pending';
  end if;
end $$;

-- Freund entfernen / Anfrage zurückziehen
create or replace function public.remove_friend(p_id bigint)
returns void language sql security definer set search_path = public as $$
  delete from friendships where id = p_id and auth.uid() in (requester, addressee);
$$;

create or replace function public.are_friends(a uuid, b uuid)
returns boolean language sql stable security definer set search_path = public as $$
  select exists (select 1 from friendships
                  where status = 'accepted'
                    and ((requester = a and addressee = b) or (requester = b and addressee = a)));
$$;

-- Chatverlauf mit einem Freund (die letzten 200, oder nur neue ab p_after)
create or replace function public.get_messages(p_friend uuid, p_after bigint default 0)
returns table (id bigint, sender uuid, receiver uuid, body text, created_at timestamptz, read_at timestamptz)
language sql stable security definer set search_path = public as $$
  select t.id, t.sender, t.receiver, t.body, t.created_at, t.read_at from (
    select m.* from messages m
     where ((m.sender = auth.uid() and m.receiver = p_friend) or (m.sender = p_friend and m.receiver = auth.uid()))
       and m.id > coalesce(p_after, 0)
     order by m.id desc
     limit 200) t
  order by t.id;
$$;

-- Nachricht senden (nur an Freunde)
create or replace function public.send_message(p_friend uuid, p_body text)
returns bigint language plpgsql security definer set search_path = public as $$
declare
  new_id bigint;
  b text := left(trim(coalesce(p_body, '')), 2000);
begin
  if auth.uid() is null or length(b) = 0 then return null; end if;
  if not are_friends(auth.uid(), p_friend) then
    raise exception 'not_friends';
  end if;
  -- Schutz vor Spam: höchstens 10 Nachrichten in 10 Sekunden
  if (select count(*) from messages where sender = auth.uid() and created_at > now() - interval '10 seconds') >= 10 then
    raise exception 'rate limit';
  end if;
  insert into messages (sender, receiver, body) values (auth.uid(), p_friend, b) returning id into new_id;
  return new_id;
end $$;

-- Nachrichten als gelesen markieren
create or replace function public.mark_read(p_friend uuid)
returns void language sql security definer set search_path = public as $$
  update messages set read_at = now() where receiver = auth.uid() and sender = p_friend and read_at is null;
$$;

-- ---------- Rechte: Funktionen nur für angemeldete Nutzer ----------
do $$
declare f text;
begin
  foreach f in array array[
    'my_profile()', 'update_profile(text, text)', 'heartbeat(text)', 'go_offline()',
    'search_users(text)', 'friends_overview()', 'send_friend_request(uuid)',
    'respond_friend_request(bigint, boolean)', 'remove_friend(bigint)', 'are_friends(uuid, uuid)',
    'get_messages(uuid, bigint)', 'send_message(uuid, text)', 'mark_read(uuid)'
  ] loop
    execute format('revoke all on function public.%s from public, anon', f);
    execute format('grant execute on function public.%s to authenticated', f);
  end loop;
end $$;
revoke all on function public.handle_new_user() from public, anon, authenticated;

-- Bereits gespeicherte, nicht vertrauenswürdige Profilbilder entfernen
update public.profiles set avatar_url = null
 where avatar_url is not null
   and avatar_url !~ '^https://(lh[0-9]\.googleusercontent\.com|avatars\.githubusercontent\.com)/';
create index if not exists messages_sender_time on public.messages (sender, created_at);
