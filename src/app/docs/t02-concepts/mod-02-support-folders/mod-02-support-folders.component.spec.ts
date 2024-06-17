import { ComponentFixture, TestBed } from '@angular/core/testing';

import { Mod02SupportFoldersComponent } from './mod-02-support-folders.component';

describe('Mod02SupportFoldersComponent', () => {
  let component: Mod02SupportFoldersComponent;
  let fixture: ComponentFixture<Mod02SupportFoldersComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [Mod02SupportFoldersComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(Mod02SupportFoldersComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
