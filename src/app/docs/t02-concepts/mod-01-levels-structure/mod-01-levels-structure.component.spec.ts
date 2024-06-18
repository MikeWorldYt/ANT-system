import { ComponentFixture, TestBed } from '@angular/core/testing';

import { Mod01LevelsStructureComponent } from './mod-01-levels-structure.component';

describe('Mod01LevelsStructureComponent', () => {
  let component: Mod01LevelsStructureComponent;
  let fixture: ComponentFixture<Mod01LevelsStructureComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [Mod01LevelsStructureComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(Mod01LevelsStructureComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
