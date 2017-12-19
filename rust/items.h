#ifndef items_h
#define items_h
#import <Foundation/Foundation.h>

struct Toodle;
struct CItem {
    char* _Nullable uuid;
    char* _Nonnull name;
    int64_t* _Nullable dueDate;
    int64_t* _Nullable completionDate;
} CF_SWIFT_NAME(ItemC);

struct CItemList {
    struct ItemC*_Nonnull* _Nonnull list;
    uint64_t* _Nonnull len;
};

struct Label;

const struct ItemC* _Nullable toodle_create_item(const struct Toodle* _Nonnull manager, const char* _Nonnull name, const int64_t* _Nullable due_date, struct Label*_Nonnull* _Nonnull list);
const void toodle_update_item(const struct Toodle* _Nonnull manager, const struct ItemC* _Nonnull item, const char* _Nullable name, const int64_t* _Nullable due_date, const int64_t* _Nullable completion_date, struct label*_Nonnull* _Nullable list);
const struct CItemList*_Nonnull toodle_get_all_items(const struct Toodle* _Nonnull manager);
const uint64_t item_list_count(const struct CItemList* _Nonnull list);
const struct ItemC* _Nullable item_list_entry_at(const struct CItemList* _Nonnull list, size_t index);

const void item_destroy(const struct CItem* _Nonnull item);

const char* _Nonnull item_get_uuid(const struct ItemC* _Nonnull item) CF_SWIFT_NAME(getter:ItemC.uuid(self:));
const char* _Nonnull item_get_name(const struct ItemC* _Nonnull item) CF_SWIFT_NAME(getter:ItemC.name(self:));
const void item_set_name(struct ItemC* _Nonnull item, const char* _Nonnull name) CF_SWIFT_NAME(setter:ItemC.name(self:newValue:));
int64_t* _Nullable item_get_due_date(const struct ItemC* _Nonnull item) CF_SWIFT_NAME(getter:ItemC.dueDate(self:));
const void item_set_due_date(struct ItemC* _Nonnull item, const int64_t* _Nullable due_date) CF_SWIFT_NAME(setter:ItemC.dueDate(self:newValue:));
int64_t* _Nullable item_get_completion_date(const struct CItem* _Nonnull item) CF_SWIFT_NAME(getter:ItemC.completionDate(self:));
const void item_set_completion_date(struct ItemC* _Nonnull item, const int64_t* _Nullable completion_date) CF_SWIFT_NAME(setter:ItemC.completionDate(self:newValue:));

#endif /* items_h */
