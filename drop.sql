DELETE FROM posts_tags;

DELETE FROM posts_authors;

DELETE FROM posts;

DELETE FROM tags;

delete from  users_migration;


delete from mobiledoc_revisions;

delete from post_revisions;


delete from users where id <> '1';
