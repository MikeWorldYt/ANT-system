import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IntoMainConceptsComponent } from './into-main-concepts.component';

describe('IntoMainConceptsComponent', () => {
  let component: IntoMainConceptsComponent;
  let fixture: ComponentFixture<IntoMainConceptsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [IntoMainConceptsComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(IntoMainConceptsComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
