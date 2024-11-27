CREATE TABLE users_migration (
    id varchar(25) PRIMARY KEY,
    user_id varchar(25) not null,
    external_id int not null,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
